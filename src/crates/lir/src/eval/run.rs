//! Every check of a program, in the LIR's order, with the engine's verdicts:
//! a `@cases` row, a scenario (its `@given` world, then its steps), a
//! property (generated arguments), a law (random traces, `law.rs`).

use super::generate::{Pools, value};
use super::machine::{Env, Flow, Halt, Machine};
use super::parallel::{RunResult, first_failure};
use super::rng::{Rng, seed_for};
use super::show::show;
use super::value::{Failure, Value};
use crate::node::Expected;
use crate::program::*;
use spec_lang_diag::{Diag, Severity};
use spec_lang_ir::TestId;
use std::collections::BTreeMap;
use std::rc::Rc;

/// Counts and the base seed, as `spec-lang test` takes them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub seed: u64,
    pub runs: u32,
    pub traces: u32,
    pub steps: u32,
    pub prefix: u32,
    pub jobs: usize,
}

impl Default for Config {
    fn default() -> Self {
        let jobs = std::thread::available_parallelism().map_or(1, usize::from);
        Config { seed: 0, runs: 100, traces: 200, steps: 40, prefix: 10, jobs }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Pass,
    Fail,
    Blocked,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Pass => "pass",
            Status::Fail => "fail",
            Status::Blocked => "blocked",
        }
    }
}

/// One check's verdict.
#[derive(Debug)]
pub struct CheckOutcome {
    pub name: String,
    pub kind: CheckKind,
    pub status: Status,
    pub diag: Option<Diag>,
}

impl CheckOutcome {
    /// Blocked by a foreign body, which only a backend runs: not red.
    pub fn blocked_by_foreign(&self) -> bool {
        self.status == Status::Blocked && self.diag.as_ref().is_some_and(|d| d.code == "B-FOREIGN")
    }
}

/// Run every check.
pub fn run_all(lir: &Lir, cfg: Config) -> Vec<CheckOutcome> {
    let mut ctx = Ctx::new(lir, cfg);
    lir.checks
        .iter()
        .map(|c| {
            let (mut status, diag) = match &c.what {
                CheckWhat::Case { target, args, expect } => match run_case(&mut ctx, *target, args, expect) {
                    Ok(()) => (Status::Pass, None),
                    Err(d) if d.code == "F-BLOCKED" => (Status::Blocked, Some(d)),
                    Err(d) => (Status::Fail, Some(d)),
                },
                CheckWhat::Test(t) => match ctx.end_of(*t) {
                    Run::Pass(_) => (Status::Pass, None),
                    Run::Fail(d) => (Status::Fail, Some(d)),
                    Run::Blocked(d) => (Status::Blocked, Some(d)),
                },
                CheckWhat::Property(t) => property(&ctx, *t),
                CheckWhat::Law(l) => super::law::run_law(&ctx, *l),
            };
            if diag.as_ref().is_some_and(|d| d.code == "B-FOREIGN") {
                status = Status::Blocked;
            }
            CheckOutcome { name: c.name.clone(), kind: c.kind, status, diag: diag.map(|d| d.in_decl(&c.decl)) }
        })
        .collect()
}

/// How one run of a test ended.
#[derive(Clone, Debug)]
pub(crate) enum Run<'l> {
    Pass(Vec<Value<'l>>),
    Fail(Diag),
    Blocked(Diag),
}

/// End worlds of tests named by `@given`, remembered.
pub(crate) struct Ctx<'l> {
    pub(crate) lir: &'l Lir,
    pub(crate) cfg: Config,
    end_worlds: BTreeMap<TestId, Run<'l>>,
    running: Vec<TestId>,
}

/// A halt as a diagnostic.
pub(crate) fn halt_diag(h: Halt<'_>, lir: &Lir) -> Diag {
    match h {
        Halt::Fatal(d) | Halt::Check(d) | Halt::Blocked(d) => *d,
        Halt::Fail(f) => {
            Diag::new(Severity::Failure, "F-TEST", format!("failed with {}", show(lir, &Value::Failure(f))))
        }
    }
}

impl<'l> Ctx<'l> {
    pub(crate) fn new(lir: &'l Lir, cfg: Config) -> Self {
        Ctx { lir, cfg, end_worlds: BTreeMap::new(), running: Vec::new() }
    }

    pub(crate) fn machine(&self) -> Result<Machine<'l>, Diag> {
        Machine::new(self.lir).map_err(|h| halt_diag(h, self.lir))
    }

    pub(crate) fn end_of(&mut self, t: TestId) -> Run<'l> {
        if let Some(r) = self.end_worlds.get(&t) {
            return r.clone();
        }
        let r = self.run_test(t, Vec::new());
        self.end_worlds.insert(t, r.clone());
        r
    }

    /// A fresh machine at a test's `@given` world.
    fn given(&mut self, t: TestId) -> Result<Machine<'l>, Run<'l>> {
        let lir = self.lir;
        let mut m = self.machine().map_err(Run::Fail)?;
        for g in &lir.test(t).given {
            match g {
                Given::State { state, value } => {
                    let v = m.eval(&mut Env::default(), value).map_err(|h| Run::Fail(halt_diag(h, lir)))?;
                    match m.conform(&v, &lir.state(*state).check) {
                        Ok(None) => m.world[state.index()] = v,
                        Ok(Some(reason)) => {
                            return Err(Run::Fail(Diag::error(
                                "E-CLAIM",
                                value.span,
                                format!("this `@given` value breaks the state's type: {reason}"),
                            )));
                        }
                        Err(h) => return Err(Run::Fail(halt_diag(h, lir))),
                    }
                }
                Given::Test(parent) => {
                    if self.running.contains(parent) || *parent == t {
                        return Err(Run::Fail(Diag::error(
                            "E-TYPE-MISMATCH",
                            lir.test(t).info.span,
                            "`@given` tests form a cycle",
                        )));
                    }
                    if !lir.test(*parent).params.is_empty() {
                        return Err(Run::Fail(Diag::error(
                            "E-TYPE-MISMATCH",
                            lir.test(t).info.span,
                            "`@given` names a test with parameters; it has no one end world",
                        )));
                    }
                    self.running.push(t);
                    let r = self.end_of(*parent);
                    self.running.pop();
                    let name = &lir.test(*parent).info.qualified;
                    match r {
                        Run::Pass(w) => m.world = w,
                        Run::Blocked(pd) if pd.code == "B-FOREIGN" => {
                            let mut d = Diag::new(
                                Severity::Blocked,
                                "B-FOREIGN",
                                format!("blocked by {name}, which reached a foreign body"),
                            );
                            d.expected = pd.expected.clone();
                            d.found = pd.found.clone();
                            return Err(Run::Blocked(d));
                        }
                        Run::Fail(_) | Run::Blocked(_) => {
                            return Err(Run::Blocked(Diag::new(
                                Severity::Failure,
                                "F-BLOCKED",
                                format!("blocked by {name}"),
                            )));
                        }
                    }
                }
            }
        }
        Ok(m)
    }

    /// Run a test with its parameters bound to `args`.
    pub(crate) fn run_test(&mut self, t: TestId, args: Vec<Value<'l>>) -> Run<'l> {
        let mut m = match self.given(t) {
            Ok(m) => m,
            Err(r) => return r,
        };
        let lir = self.lir;
        let test = lir.test(t);
        let mut env = Env::default();
        for (p, v) in test.params.iter().zip(args) {
            env.push(&p.name, v);
        }
        for s in &test.body.stmts {
            match m.exec(&mut env, s) {
                Ok(Flow::Normal) => {}
                Ok(Flow::Return(_)) => break,
                Err(Halt::Blocked(d)) => return Run::Blocked(*d),
                Err(Halt::Fail(f)) => {
                    return Run::Fail(
                        Diag::new(
                            Severity::Failure,
                            "F-TEST",
                            format!("this step was refused: {}", show(lir, &Value::Failure(f))),
                        )
                        .with_span(s.span),
                    );
                }
                Err(h) => return Run::Fail(halt_diag(h, lir)),
            }
        }
        Run::Pass(m.world)
    }
}

/// One `@cases` row.
fn run_case<'l>(
    ctx: &mut Ctx<'l>,
    target: CaseTarget,
    args: &'l [Option<crate::node::Expr>],
    expect: &'l Expected,
) -> Result<(), Diag> {
    let lir = ctx.lir;
    let mut m = match target {
        CaseTarget::Test(t) => match ctx.given(t) {
            Ok(m) => m,
            Err(Run::Fail(d) | Run::Blocked(d)) => return Err(d),
            Err(Run::Pass(_)) => return Ok(()),
        },
        CaseTarget::Fn(_) => ctx.machine()?,
    };
    let mut vals = Vec::new();
    for a in args {
        vals.push(match a {
            Some(e) => Some(m.eval(&mut Env::default(), e).map_err(|h| halt_diag(h, lir))?),
            None => None,
        });
    }
    let got = match target {
        CaseTarget::Test(t) => {
            return match ctx.run_test(t, vals.into_iter().flatten().collect()) {
                Run::Pass(_) => Ok(()),
                Run::Fail(d) | Run::Blocked(d) => Err(d),
            };
        }
        CaseTarget::Fn(f) => m.call_fn(f, vals),
    };
    let got = match got {
        Ok(v) => Ok(v),
        Err(Halt::Fail(f)) => Err(f),
        Err(h) => return Err(halt_diag(h, lir)),
    };
    let want = match expect {
        Expected::Value(e) => Ok(m.eval(&mut Env::default(), e).map_err(|h| halt_diag(h, lir))?),
        Expected::Fail { name, .. } => Err(Rc::new(Failure { name: name.clone(), data: Vec::new() })),
    };
    let same = match (&got, &want) {
        (Ok(a), Ok(b)) => a == b,
        (Err(a), Err(b)) => a.name == b.name,
        _ => false,
    };
    if same {
        return Ok(());
    }
    let text = |r: &Result<Value<'l>, Rc<Failure<'l>>>| match r {
        Ok(v) => show(lir, v),
        Err(f) => show(lir, &Value::Failure(f.clone())),
    };
    Err(Diag::new(Severity::Failure, "F-CASE", format!("the case gave {}", text(&got)))
        .with_expected(text(&want), text(&got)))
}

/// A property: `runs` runs, each with arguments drawn from seed
/// `seed_for(seed, decl, run)`; the first run that does not pass decides.
fn property(ctx: &Ctx<'_>, t: TestId) -> (Status, Option<Diag>) {
    let lir = ctx.lir;
    let cfg = ctx.cfg;
    let decl = &lir.test(t).info.qualified;
    let merged = first_failure(cfg.runs, cfg.jobs, |run| {
        let mut ctx = Ctx::new(lir, cfg);
        let failure = match args_of(&ctx, t, decl, run) {
            Err(d) => Some((Status::Fail, d)),
            Ok(args) => match ctx.run_test(t, args) {
                Run::Pass(_) => None,
                Run::Blocked(d) => Some((Status::Blocked, d)),
                Run::Fail(d) => Some((
                    Status::Fail,
                    d.with_note(format!("run {run}, seed {}", seed_for(cfg.seed, decl, u64::from(run)))),
                )),
            },
        };
        RunResult { failure }
    });
    match merged.first {
        None => (Status::Pass, None),
        Some((_, (s, d))) => (s, Some(d)),
    }
}

fn args_of<'l>(ctx: &Ctx<'l>, t: TestId, decl: &str, run: u32) -> Result<Vec<Value<'l>>, Diag> {
    let mut rng = Rng::new(seed_for(ctx.cfg.seed, decl, u64::from(run)));
    let mut m = ctx.machine()?;
    let mut out = Vec::new();
    for p in &ctx.lir.test(t).params {
        out.push(
            value(&mut m, &mut rng, &Pools::default(), &p.check).map_err(|e| e.into_diag(ctx.lir.test(t).info.span))?,
        );
    }
    Ok(out)
}
