//! The check runner: every check of the program, with the engine's verdicts. A `@cases` row, a scenario (its
//! `@given` world, then its steps), a property (generated arguments, the lowest failing run decides) and a law
//! (random traces drawn as the engine draws them). Prints one JSON line per check.

use super::rng::{Rng, seed_for};
use super::{Halt, R, Show};
use crate::checks::{CHECKS, LAWS, TESTS};
use crate::machine::{Event, Machine, Pools, Reply, World, random_world};
use std::collections::BTreeMap;

/// A test's steps with its arguments bound, run on its `@given` world.
pub type Body = Box<dyn FnOnce(&mut Machine) -> R<()>>;
/// A law's body with its other parameters drawn, given the trace.
pub type LawBody = Box<dyn FnOnce(&mut Machine, Trace) -> R<bool>>;

/// One `@given` item.
#[derive(Debug)]
pub enum Given {
    /// Sets a state: evaluates its value, checks it against the state's type, writes it.
    State(fn(&mut Machine) -> R<()>),
    /// The end world of another test, by index into `TESTS`.
    Test(usize),
}

#[derive(Debug)]
pub struct TestDecl {
    pub name: &'static str,
    pub has_params: bool,
    pub given: &'static [Given],
    /// The steps, for a test without parameters.
    pub run: fn(&mut Machine) -> R<()>,
}

#[derive(Debug)]
pub struct LawDecl {
    pub name: &'static str,
    /// The rules a trace calls and the rules that may build its start world, by `Event::random`'s numbering.
    pub traced: &'static [usize],
    pub start: &'static [usize],
    pub draw: fn(&mut Machine, &mut Rng, &Pools) -> R<LawBody>,
}

#[derive(Debug)]
pub enum What {
    /// A `@cases` row of a `fn`: true when it gave what the row expects.
    Case(fn(&mut Machine) -> R<bool>),
    /// A `@cases` row of a test: its arguments, evaluated on the test's `@given` world.
    TestCase {
        test: usize,
        args: fn(&mut Machine) -> R<Body>,
    },
    Test(usize),
    Property {
        test: usize,
        draw: fn(&mut Machine, &mut Rng) -> R<Body>,
    },
    Law(usize),
}

#[derive(Debug)]
pub struct Check {
    pub name: &'static str,
    pub what: What,
}

/// One step of a trace.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Step {
    pub event: Event,
    pub reply: Reply,
    pub before: World,
    pub after: World,
}

/// A trace: its steps, and the start world then the world after each step.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Trace {
    pub steps: Vec<Step>,
    pub states: Vec<World>,
}

impl Show for Step {
    fn show(&self) -> String {
        format!("{} -> {}", self.event.show(), self.reply.show())
    }
}

impl Show for Trace {
    fn show(&self) -> String {
        format!("<trace of {} steps>", self.steps.len())
    }
}

/// Counts and the base seed, as `spec-lang test` takes them.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub seed: u64,
    pub runs: u32,
    pub traces: u32,
    pub steps: u32,
    pub prefix: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config { seed: 0, runs: 100, traces: 200, steps: 40, prefix: 10 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Pass,
    Fail,
    Blocked,
}

/// Why a check did not pass.
#[derive(Clone, Debug)]
pub struct Note {
    pub code: &'static str,
    pub message: String,
}

impl Note {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Note { code, message: message.into() }
    }

    fn of(h: &Halt) -> Self {
        Note { code: h.code(), message: h.message() }
    }

    /// A halt while generating: a failure there is an unmet claim.
    fn of_gen(h: &Halt) -> Self {
        match h {
            Halt::Fail(f) => Note::new("E-GEN", format!("a claim failed with {} while generating", f.name)),
            h => Note::of(h),
        }
    }
}

/// How one run of a test ended.
#[derive(Clone, Debug)]
enum Run {
    Pass(Box<World>),
    Fail(Note),
    Blocked(Note),
}

/// End worlds of tests named by `@given`, remembered.
struct Runner {
    cfg: Config,
    ends: BTreeMap<usize, Run>,
    running: Vec<usize>,
}

impl Runner {
    fn new(cfg: Config) -> Self {
        Runner { cfg, ends: BTreeMap::new(), running: Vec::new() }
    }

    fn end_of(&mut self, t: usize) -> Run {
        if let Some(r) = self.ends.get(&t) {
            return r.clone();
        }
        let r = self.run_test(t, Box::new(TESTS[t].run));
        self.ends.insert(t, r.clone());
        r
    }

    /// A fresh machine at a test's `@given` world.
    fn given(&mut self, t: usize) -> Result<Machine, Box<Run>> {
        let mut m = Machine::new().map_err(|h| Box::new(Run::Fail(Note::of(&h))))?;
        for g in TESTS[t].given {
            match g {
                Given::State(set) => set(&mut m).map_err(|h| Box::new(Run::Fail(Note::of(&h))))?,
                Given::Test(parent) => {
                    if self.running.contains(parent) || *parent == t {
                        return Err(Box::new(Run::Fail(Note::new("E-TYPE-MISMATCH", "`@given` tests form a cycle"))));
                    }
                    if TESTS[*parent].has_params {
                        return Err(Box::new(Run::Fail(Note::new(
                            "E-TYPE-MISMATCH",
                            "`@given` names a test with parameters; it has no one end world",
                        ))));
                    }
                    self.running.push(t);
                    let r = self.end_of(*parent);
                    self.running.pop();
                    let name = TESTS[*parent].name;
                    match r {
                        Run::Pass(w) => m.world = *w,
                        Run::Blocked(n) if n.code == "B-FOREIGN" => {
                            return Err(Box::new(Run::Blocked(Note::new(
                                "B-FOREIGN",
                                format!("blocked by {name}, which reached a foreign body"),
                            ))));
                        }
                        Run::Fail(_) | Run::Blocked(_) => {
                            return Err(Box::new(Run::Blocked(Note::new("F-BLOCKED", format!("blocked by {name}")))));
                        }
                    }
                }
            }
        }
        Ok(m)
    }

    fn run_test(&mut self, t: usize, body: Body) -> Run {
        let mut m = match self.given(t) {
            Ok(m) => m,
            Err(r) => return *r,
        };
        match body(&mut m) {
            Ok(()) => Run::Pass(Box::new(m.world)),
            Err(h @ Halt::Blocked { .. }) => Run::Blocked(Note::of(&h)),
            Err(Halt::Fail(f)) => Run::Fail(Note::new("F-TEST", format!("this step was refused: {}", f.show()))),
            Err(h) => Run::Fail(Note::of(&h)),
        }
    }

    fn case(&mut self, what: &What) -> Result<(), Note> {
        match what {
            What::Case(row) => {
                let mut m = Machine::new().map_err(|h| Note::of(&h))?;
                match row(&mut m) {
                    Ok(true) => Ok(()),
                    Ok(false) => Err(Note::new("F-CASE", "the case gave another value")),
                    Err(h) => Err(Note::of(&h)),
                }
            }
            What::TestCase { test, args } => {
                let mut m = match self.given(*test) {
                    Ok(m) => m,
                    Err(r) => match *r {
                        Run::Fail(n) | Run::Blocked(n) => return Err(n),
                        Run::Pass(_) => return Ok(()),
                    },
                };
                let body = args(&mut m).map_err(|h| Note::of(&h))?;
                match self.run_test(*test, body) {
                    Run::Pass(_) => Ok(()),
                    Run::Fail(n) | Run::Blocked(n) => Err(n),
                }
            }
            _ => Ok(()),
        }
    }

    fn property(&self, t: usize, draw: fn(&mut Machine, &mut Rng) -> R<Body>) -> (Status, Option<Note>) {
        let name = TESTS[t].name;
        for run in 0..self.cfg.runs {
            let mut rng = Rng::new(seed_for(self.cfg.seed, name, u64::from(run)));
            let body = Machine::new()
                .map_err(|h| Note::of(&h))
                .and_then(|mut m| draw(&mut m, &mut rng).map_err(|h| Note::of_gen(&h)));
            let failure = match body {
                Err(n) => Some((Status::Fail, n)),
                Ok(body) => match Runner::new(self.cfg).run_test(t, body) {
                    Run::Pass(_) => None,
                    Run::Blocked(n) => Some((Status::Blocked, n)),
                    Run::Fail(n) => Some((Status::Fail, n)),
                },
            };
            if let Some((s, n)) = failure {
                return (s, Some(n));
            }
        }
        (Status::Pass, None)
    }
}

/// One executed trace.
struct TraceRun {
    steps: Vec<(Event, Reply)>,
    worlds: Vec<World>,
}

impl TraceRun {
    /// The trace of its first `n` steps.
    fn value(&self, n: usize) -> Trace {
        let steps = self
            .steps
            .iter()
            .take(n)
            .enumerate()
            .map(|(i, (e, r))| Step {
                event: e.clone(),
                reply: r.clone(),
                before: self.worlds[i].clone(),
                after: self.worlds[i + 1].clone(),
            })
            .collect();
        let states = self.worlds.iter().take(n + 1).cloned().collect();
        Trace { steps, states }
    }
}

enum Verdict {
    Holds,
    False,
    Broken(Note),
}

fn verdict(body: LawBody, trace: Trace) -> Verdict {
    let mut m = match Machine::new() {
        Ok(m) => m,
        Err(h) => return Verdict::Broken(Note::of(&h)),
    };
    match body(&mut m, trace) {
        Ok(true) => Verdict::Holds,
        Ok(false) => Verdict::False,
        Err(Halt::Fail(f)) => {
            Verdict::Broken(Note::new("F-LAW", format!("the law failed with {} instead of giving a Bool", f.name)))
        }
        Err(h) => Verdict::Broken(Note::of(&h)),
    }
}

struct Laws {
    cfg: Config,
}

impl Laws {
    fn run(&self, l: usize) -> (Status, Option<Note>) {
        let law = &LAWS[l];
        for i in 0..self.cfg.traces {
            let failure = match self.one_trace(law, i) {
                Err(n) => Some(n),
                Ok((run, body)) => match verdict(body, run.value(run.steps.len())) {
                    Verdict::Holds => None,
                    Verdict::Broken(n) => Some(n),
                    Verdict::False => Some(Note::new(
                        "F-LAW",
                        format!(
                            "law {} is false on trace {i}, seed {}",
                            law.name,
                            seed_for(self.cfg.seed, law.name, u64::from(i))
                        ),
                    )),
                },
            };
            if let Some(n) = failure {
                return (Status::Fail, Some(n));
            }
        }
        (Status::Pass, None)
    }

    fn one_trace(&self, law: &LawDecl, i: u32) -> Result<(TraceRun, LawBody), Note> {
        let mut rng = Rng::new(seed_for(self.cfg.seed, law.name, u64::from(i)));
        let start = if i % 2 == 1 { self.lawful_world(&mut rng)? } else { None };
        let run = self.generate(&mut rng, law.traced, law.start, start)?;
        let body = other_params(law, &run, &mut rng)?;
        Ok((run, body))
    }

    /// A world drawn from the states' declared types on which every law holds at the start.
    fn lawful_world(&self, rng: &mut Rng) -> Result<Option<World>, Note> {
        let mut m = Machine::new().map_err(|h| Note::of(&h))?;
        for _ in 0..200 {
            let w = random_world(&mut m, rng).map_err(|h| Note::of_gen(&h))?;
            let run = TraceRun { steps: Vec::new(), worlds: vec![w.clone()] };
            if is_lawful(&run, rng)? {
                return Ok(Some(w));
            }
        }
        Ok(None)
    }

    fn generate(
        &self,
        rng: &mut Rng,
        traced: &[usize],
        prefix: &[usize],
        start: Option<World>,
    ) -> Result<TraceRun, Note> {
        let mut m = Machine::new().map_err(|h| Note::of(&h))?;
        let n_prefix = match start {
            Some(w) => {
                m.world = w;
                0
            }
            None => rng.below(u64::from(self.cfg.prefix) + 1),
        };
        for _ in 0..n_prefix {
            let e = random_event(&mut m, rng, prefix)?;
            e.call(&mut m).map_err(|h| Note::of(&h))?;
        }
        let mut run = TraceRun { steps: Vec::new(), worlds: vec![m.world.clone()] };
        for _ in 0..self.cfg.steps {
            let e = random_event(&mut m, rng, traced)?;
            let r = e.call(&mut m).map_err(|h| Note::of(&h))?;
            run.steps.push((e, r));
            run.worlds.push(m.world.clone());
        }
        Ok(run)
    }
}

fn is_lawful(run: &TraceRun, rng: &mut Rng) -> Result<bool, Note> {
    for law in LAWS {
        let body = other_params(law, run, rng)?;
        if !matches!(verdict(body, run.value(0)), Verdict::Holds) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// The parameters besides the trace, drawn with pools of every world of the trace.
fn other_params(law: &LawDecl, run: &TraceRun, rng: &mut Rng) -> Result<LawBody, Note> {
    let pools = Pools::of(&run.worlds);
    let mut m = Machine::new().map_err(|h| Note::of(&h))?;
    (law.draw)(&mut m, rng, &pools).map_err(|h| Note::of_gen(&h))
}

fn random_event(m: &mut Machine, rng: &mut Rng, rules: &[usize]) -> Result<Event, Note> {
    let Some(&rule) = rng.pick(rules) else {
        return Err(Note::new("E-GEN", "a trace with no rules"));
    };
    let pools = Pools::of(std::slice::from_ref(&m.world));
    Event::random(m, rng, &pools, rule).map_err(|h| Note::of_gen(&h))
}

/// Every check, in order, with its status.
pub fn run_all(cfg: Config) -> Vec<(&'static str, Status, Option<Note>)> {
    let mut runner = Runner::new(cfg);
    CHECKS
        .iter()
        .map(|c| {
            let (mut status, note) = match &c.what {
                What::Case(_) | What::TestCase { .. } => match runner.case(&c.what) {
                    Ok(()) => (Status::Pass, None),
                    Err(n) if n.code == "F-BLOCKED" => (Status::Blocked, Some(n)),
                    Err(n) => (Status::Fail, Some(n)),
                },
                What::Test(t) => match runner.end_of(*t) {
                    Run::Pass(_) => (Status::Pass, None),
                    Run::Fail(n) => (Status::Fail, Some(n)),
                    Run::Blocked(n) => (Status::Blocked, Some(n)),
                },
                What::Property { test, draw } => runner.property(*test, *draw),
                What::Law(l) => Laws { cfg }.run(*l),
            };
            if note.as_ref().is_some_and(|n| n.code == "B-FOREIGN") {
                status = Status::Blocked;
            }
            (c.name, status, note)
        })
        .collect()
}

fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `bin/test`: `[--seed N] [--runs N] [--traces N] [--steps N]`; one JSON line per check. Exit 1 when a check
/// fails or a blocked check is not blocked by a foreign body.
pub fn main() -> std::process::ExitCode {
    let mut cfg = Config::default();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let value = it.next().and_then(|v| v.parse::<u64>().ok());
        let small = value.and_then(|v| u32::try_from(v).ok());
        match (flag.as_str(), value, small) {
            ("--seed", Some(v), _) => cfg.seed = v,
            ("--runs", _, Some(v)) => cfg.runs = v,
            ("--traces", _, Some(v)) => cfg.traces = v,
            ("--steps", _, Some(v)) => cfg.steps = v,
            _ => {
                eprintln!("usage: test [--seed N] [--runs N] [--traces N] [--steps N]");
                return std::process::ExitCode::from(2);
            }
        }
    }
    let mut green = true;
    for (name, status, note) in run_all(cfg) {
        let status = match status {
            Status::Pass => "pass",
            Status::Fail => "fail",
            Status::Blocked => "blocked",
        };
        green &= status == "pass" || note.as_ref().is_some_and(|n| n.code == "B-FOREIGN");
        let note = note.map_or(String::new(), |n| {
            format!(", \"code\": {}, \"message\": {}", json_str(n.code), json_str(&n.message))
        });
        println!("{{\"check\": {}, \"status\": \"{status}\"{note}}}", json_str(name));
    }
    if green { std::process::ExitCode::SUCCESS } else { std::process::ExitCode::FAILURE }
}

/// Whether every law of the program holds on this world alone, other parameters drawn from seed 0: what a world
/// read from outside (a store) must meet.
pub fn lawful(world: &World) -> bool {
    let run = TraceRun { steps: Vec::new(), worlds: vec![world.clone()] };
    is_lawful(&run, &mut Rng::new(0)).unwrap_or(false)
}
