//! Laws over random traces, with a shrunk counterexample written as a test.

use crate::generate::{Pools, value};
use crate::parallel::{RunResult, first_failure};
use crate::report::Status;
use crate::rng::{Rng, seed_for};
use crate::scenario::{Ctx, halt_diag};
use crate::shrink::candidates;
use crate::trace::{TraceRun, generate, prefix_rules, replay};
use spec_lang_diag::{Diag, Severity};
use spec_lang_engine::display::{call, show};
use spec_lang_engine::{Env, Event, Halt, Value, World};
use spec_lang_ir::*;
use std::rc::Rc;

/// What one evaluation of a law gave.
enum Verdict {
    Holds,
    False,
    Broken(Diag),
}

/// Evaluate the law on the first `n` steps of a run, other parameters fixed.
fn verdict(ctx: &Ctx<'_>, law: &LawDecl, run: &TraceRun, n: usize, others: &[(String, Value)]) -> Verdict {
    let Some((ti, _)) = law.trace() else {
        return Verdict::Broken(Diag::error("E-TYPE-MISMATCH", law.info.span, "a law needs a Trace parameter"));
    };
    let mut it = match ctx.interp() {
        Ok(it) => it,
        Err(d) => return Verdict::Broken(d),
    };
    let mut env = Env::default();
    let mut others = others.iter();
    for (i, p) in law.params.iter().enumerate() {
        if i == ti {
            env.push(&p.name, run.value(n));
        } else if let Some((name, v)) = others.next() {
            env.push(name, v.clone());
        }
    }
    match it.eval_in(&mut env, &law.body) {
        Ok(Value::Bool(true)) => Verdict::Holds,
        Ok(Value::Bool(false)) => Verdict::False,
        Ok(v) => Verdict::Broken(
            Diag::error("E-TYPE-MISMATCH", law.body.span, "a law is a Bool").with_expected("Bool", show(ctx.prog, &v)),
        ),
        Err(Halt::Fail(f)) => Verdict::Broken(
            Diag::new(Severity::Failure, "F-LAW", format!("the law failed with {} instead of giving a Bool", f.name))
                .with_span(law.body.span),
        ),
        Err(h) => Verdict::Broken(halt_diag(ctx.prog, h, Some(law.body.span))),
    }
}

/// Check a law; `lawful` counts the traces that started from a lawful random world.
pub(crate) fn run_law(ctx: &mut Ctx<'_>, l: LawId, lawful: &mut u32) -> (Status, Option<Diag>) {
    let prog = ctx.prog;
    let law = prog.law(l);
    let decl = prog.qualified(&law.info);
    let Some((ti, traced)) = law.trace() else {
        return (
            Status::Fail,
            Some(Diag::error("E-TYPE-MISMATCH", law.info.span, "a law needs a Trace parameter").with_decl(decl)),
        );
    };
    let prefix = prefix_rules(prog, traced);
    let cfg = ctx.cfg;
    // Every trace on a worker with its own interpreter; merged by trace number.
    let merged = first_failure(cfg.traces, cfg.jobs, |i| {
        let ctx = Ctx::new(prog, cfg);
        match one_trace(&ctx, law, &decl, ti, traced, &prefix, i) {
            Err(d) => RunResult { tally: 0, failure: Some(LawFailure::Broken(d)) },
            Ok(t) => {
                let failure = match verdict(&ctx, law, &t.run, t.run.steps.len(), &t.others) {
                    Verdict::Holds => None,
                    Verdict::Broken(d) => Some(LawFailure::Broken(d.in_decl(&decl).with_note(t.at()))),
                    Verdict::False => Some(LawFailure::False),
                };
                RunResult { tally: u32::from(t.run.start.is_some()), failure }
            }
        }
    });
    *lawful = merged.tally;
    match merged.first {
        None => (Status::Pass, None),
        Some((_, LawFailure::Broken(d))) => (Status::Fail, Some(d)),
        Some((i, LawFailure::False)) => {
            // Run that trace again here, where its values can live, and shrink it.
            let t = match one_trace(ctx, law, &decl, ti, traced, &prefix, i) {
                Ok(t) => t,
                Err(d) => return (Status::Fail, Some(d)),
            };
            let total = t.run.prefix.len() + t.run.steps.len();
            let run = shrink(ctx, law, t.run, &t.others);
            let mut d = counterexample(ctx, law, &run, &t.others, total);
            if let Some(w) = &run.start {
                d = crate::missing::offer(ctx, law, w, d);
            }
            let d = d
                .in_decl(&decl)
                .with_note(format!("trace {i} of {}, seed {} (base seed {})", cfg.traces, t.seed, cfg.seed));
            (Status::Fail, Some(d))
        }
    }
}

/// Why a trace did not hold. Values stay on their thread; a false law is
/// replayed from its number on the calling thread.
enum LawFailure {
    Broken(Diag),
    False,
}

/// One generated trace and the law's other parameters.
struct OneTrace {
    run: TraceRun,
    others: Vec<(String, Value)>,
    index: u32,
    seed: u64,
}

impl OneTrace {
    fn at(&self) -> String {
        format!("trace {}, seed {}", self.index, self.seed)
    }
}

/// Trace number `i` of a law: a pure function of the base seed, the law's
/// name and `i`. Odd traces start from a random lawful world, even ones
/// from a reached one.
fn one_trace(
    ctx: &Ctx<'_>,
    law: &LawDecl,
    decl: &str,
    ti: usize,
    traced: &[RuleId],
    prefix: &[RuleId],
    i: u32,
) -> Result<OneTrace, Diag> {
    let seed = seed_for(ctx.cfg.seed, decl, u64::from(i));
    let mut rng = Rng::new(seed);
    let start = if i % 2 == 1 { lawful_world(ctx, &mut rng).map_err(|d| d.in_decl(decl))? } else { None };
    let run = generate(ctx, &mut rng, traced, prefix, start)
        .map_err(|d| d.in_decl(decl).with_note(format!("trace {i}, seed {seed}")))?;
    let others = other_params(ctx, law, ti, &run, &mut rng).map_err(|d| d.in_decl(decl))?;
    Ok(OneTrace { run, others, index: i, seed })
}

/// Tries to draw one lawful start world.
const WORLD_TRIES: u32 = 200;

/// A world drawn from the states' types (claims included) on which every
/// law of the program holds: the card's "any world its types, flows and laws
/// allow". `None` when no try met every law; the trace then starts reached.
fn lawful_world(ctx: &Ctx<'_>, rng: &mut Rng) -> Result<Option<World>, Diag> {
    let prog = ctx.prog;
    let mut it = ctx.interp()?;
    for _ in 0..WORLD_TRIES {
        let mut states = Vec::new();
        for s in &prog.states {
            states.push(value(&mut it, rng, &Pools::default(), &s.ty).map_err(|e| e.into_diag(s.info.span))?);
        }
        let w = World { states };
        let run = TraceRun { start: Some(w.clone()), prefix: Vec::new(), steps: Vec::new(), worlds: vec![w.clone()] };
        if is_lawful(ctx, &run, rng)? {
            return Ok(Some(w));
        }
    }
    Ok(None)
}

/// Every law of the program holds on the run's start world alone.
fn is_lawful(ctx: &Ctx<'_>, run: &TraceRun, rng: &mut Rng) -> Result<bool, Diag> {
    for law in &ctx.prog.laws {
        let Some((ti, _)) = law.trace() else { continue };
        let others = other_params(ctx, law, ti, run, rng)?;
        if !matches!(verdict(ctx, law, run, 0, &others), Verdict::Holds) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// A smaller lawful start world on which the same calls still break the law.
fn shrink_start(
    ctx: &Ctx<'_>,
    law: &LawDecl,
    mut best: TraceRun,
    others: &[(String, Value)],
    budget: &mut u32,
) -> TraceRun {
    let Some(start) = best.start.clone() else { return best };
    let steps: Vec<Rc<Event>> = best.steps.iter().map(|(e, _)| e.clone()).collect();
    let mut world = start;
    let mut changed = true;
    while changed && *budget > 0 {
        changed = false;
        for i in 0..world.states.len() {
            for c in candidates(&world.states[i]) {
                if *budget == 0 {
                    return best;
                }
                *budget -= 1;
                let Ok(mut it) = ctx.interp() else { return best };
                if !matches!(it.conform(&c, &ctx.prog.states[i].ty), Ok(None)) {
                    continue;
                }
                let mut w = world.clone();
                w.states[i] = c;
                let alone =
                    TraceRun { start: Some(w.clone()), prefix: Vec::new(), steps: Vec::new(), worlds: vec![w.clone()] };
                if !matches!(is_lawful(ctx, &alone, &mut Rng::new(0)), Ok(true)) {
                    continue;
                }
                if let Ok(r) = replay(ctx, &Some(w.clone()), &[], &steps)
                    && matches!(verdict(ctx, law, &r, r.steps.len(), others), Verdict::False)
                {
                    world = w;
                    best = r;
                    changed = true;
                    break;
                }
            }
        }
    }
    best
}

/// Parameters besides the trace, generated after it so numbers can lean on
/// keys that occur somewhere in the trace.
fn other_params(
    ctx: &Ctx<'_>,
    law: &LawDecl,
    ti: usize,
    run: &TraceRun,
    rng: &mut Rng,
) -> Result<Vec<(String, Value)>, Diag> {
    let pools = Pools::from_worlds(ctx.prog, run.worlds.iter());
    let mut it = ctx.interp()?;
    let mut out = Vec::new();
    for (i, p) in law.params.iter().enumerate() {
        if i != ti {
            out.push((p.name.clone(), value(&mut it, rng, &pools, &p.ty).map_err(|e| e.into_diag(p.span))?));
        }
    }
    Ok(out)
}

/// Drop calls, prefix and trace alike, while the law stays false.
fn shrink(ctx: &Ctx<'_>, law: &LawDecl, run: TraceRun, others: &[(String, Value)]) -> TraceRun {
    let mut prefix: Vec<Rc<Event>> = run.prefix.iter().map(|(e, _)| e.clone()).collect();
    let mut steps: Vec<Rc<Event>> = run.steps.iter().map(|(e, _)| e.clone()).collect();
    let mut best = run;
    let mut budget: u32 = 400;
    let mut changed = true;
    while changed && budget > 0 {
        changed = false;
        for in_prefix in [false, true] {
            let mut i = 0;
            loop {
                let len = if in_prefix { prefix.len() } else { steps.len() };
                if i >= len {
                    break;
                }
                let (mut p, mut s) = (prefix.clone(), steps.clone());
                if in_prefix {
                    p.remove(i);
                } else {
                    s.remove(i);
                }
                budget = budget.saturating_sub(1);
                match replay(ctx, &best.start, &p, &s) {
                    Ok(r) if matches!(verdict(ctx, law, &r, r.steps.len(), others), Verdict::False) => {
                        prefix = p;
                        steps = s;
                        best = r;
                        changed = true;
                    }
                    _ => i += 1,
                }
            }
        }
        // Then a smaller start world, then smaller arguments.
        best = shrink_start(ctx, law, best, others, &mut budget);
        // Smaller arguments, one at a time.
        for in_prefix in [false, true] {
            let len = if in_prefix { prefix.len() } else { steps.len() };
            for i in 0..len {
                let e = if in_prefix { prefix[i].clone() } else { steps[i].clone() };
                let params = &ctx.prog.rule(e.rule).params;
                for (a, p) in params.iter().enumerate() {
                    for c in candidates(&e.args[a]) {
                        if budget == 0 {
                            return best;
                        }
                        budget -= 1;
                        let Ok(mut it) = ctx.interp() else { return best };
                        if !matches!(it.conform(&c, &p.ty), Ok(None)) {
                            continue;
                        }
                        let mut args = e.args.clone();
                        args[a] = c;
                        let smaller = Rc::new(Event { rule: e.rule, args });
                        let (mut pr, mut st) = (prefix.clone(), steps.clone());
                        if in_prefix {
                            pr[i] = smaller.clone();
                        } else {
                            st[i] = smaller.clone();
                        }
                        if let Ok(r) = replay(ctx, &best.start, &pr, &st)
                            && matches!(verdict(ctx, law, &r, r.steps.len(), others), Verdict::False)
                        {
                            prefix = pr;
                            steps = st;
                            best = r;
                            changed = true;
                            break;
                        }
                    }
                }
            }
        }
    }
    best
}

/// A step as a test writes it: `call;`, `call -> v;` or `call -> fail E;`.
fn step_line(ctx: &Ctx<'_>, e: &Event, reply: &Value) -> String {
    let c = call(ctx.prog, e);
    let inner = match reply {
        Value::Variant(_, _, xs) => xs.first().cloned().unwrap_or(Value::Unit),
        other => other.clone(),
    };
    match inner {
        Value::Unit => format!("{c};"),
        v => format!("{c} -> {};", show(ctx.prog, &v)),
    }
}

/// The counterexample as a pasteable test, where it broke, and what changed.
fn counterexample(ctx: &Ctx<'_>, law: &LawDecl, run: &TraceRun, others: &[(String, Value)], from: usize) -> Diag {
    let prog = ctx.prog;
    let n = (0..=run.steps.len())
        .find(|n| matches!(verdict(ctx, law, run, *n, others), Verdict::False))
        .unwrap_or(run.steps.len());
    let mut test = String::new();
    if let Some(w) = &run.start {
        let items: Vec<String> = w
            .states
            .iter()
            .enumerate()
            .map(|(i, v)| format!("  {} -> {},", prog.qualified(&prog.states[i].info), show(prog, v)))
            .collect();
        test.push_str(&format!("@given(\n{}\n)\n", items.join("\n")));
    }
    test.push_str(&format!("test {}_counterexample = {{\n", law.info.name));
    for (e, r) in &run.prefix {
        test.push_str(&format!("  {}\n", step_line(ctx, e, r)));
    }
    test.push_str("  // the trace starts here\n");
    for (e, r) in &run.steps {
        test.push_str(&format!("  {}\n", step_line(ctx, e, r)));
    }
    test.push('}');
    let size = run.prefix.len() + run.steps.len();
    let from_where = if run.start.is_some() { "from a random lawful world" } else { "from a reached world" };
    let mut d = Diag::new(
        Severity::Failure,
        "F-LAW",
        format!("law {} is false after {} call(s) {from_where}", law.info.name, size),
    )
        .with_span(law.body.span)
        .with_note(match run.start {
            Some(_) => "start: a random world its types and every law allow; no rule call was searched for that reaches it, so a missing state law is as likely as a bug".to_string(),
            None => "start: the initial world, then the calls above the trace".to_string(),
        })
        .with_note(format!("shrunk from {from} to {size} call(s):\n{test}"));
    if !others.is_empty() {
        let shown: Vec<String> = others.iter().map(|(k, v)| format!("{k} = {}", show(prog, v))).collect();
        d = d.with_note(format!("with {}", shown.join(", ")));
    }
    if n == 0 {
        d = d.with_note("false already at the start world");
    } else {
        let (e, r) = &run.steps[n - 1];
        d = d.with_note(format!("broke at step {n}: {}", step_line(ctx, e, r)));
        d = d.with_note(format!("changed: {}", diff(prog, &run.worlds[n - 1], &run.worlds[n])));
    }
    d
}

/// The state entries a step changed: `board.tasks[2]: absent -> Task(...)`.
fn diff(prog: &Program, before: &World, after: &World) -> String {
    let mut out = Vec::new();
    for (i, (b, a)) in before.states.iter().zip(&after.states).enumerate() {
        if b == a {
            continue;
        }
        let name = prog.qualified(&prog.states[i].info);
        match (b, a) {
            (Value::Map(mb), Value::Map(ma)) => {
                let keys: std::collections::BTreeSet<&Value> = mb.keys().chain(ma.keys()).collect();
                for k in keys {
                    let (x, y) = (mb.get(k), ma.get(k));
                    if x != y {
                        let s = |v: Option<&Value>| v.map_or("absent".to_string(), |v| show(prog, v));
                        out.push(format!("{name}[{}]: {} -> {}", show(prog, k), s(x), s(y)));
                    }
                }
            }
            _ => out.push(format!("{name}: {} -> {}", show(prog, b), show(prog, a))),
        }
    }
    if out.is_empty() { "nothing".to_string() } else { out.join("; ") }
}
