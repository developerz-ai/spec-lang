//! Laws over random traces, drawn as the engine draws them: trace `i` of a
//! law uses seed `seed_for(seed, law, i)`; odd traces start from a random
//! lawful world, even ones from the initial world after up to `prefix`
//! random calls of the law's start rules; then `steps` random calls of the
//! traced rules; then the other parameters. A false law is reported, not
//! shrunk (the engine's shrinking changes the message, never the verdict).

use super::generate::{Pools, value};
use super::machine::{Env, Machine};
use super::parallel::{RunResult, first_failure};
use super::rng::{Rng, seed_for};
use super::run::{Ctx, Status, halt_diag};
use super::value::{Event, Step, Trace, Value};
use crate::program::Law;
use spec_lang_diag::{Diag, Severity};
use spec_lang_ir::{LawId, RuleId};
use std::rc::Rc;

/// Tries to draw one lawful start world.
const WORLD_TRIES: u32 = 200;

/// One executed trace.
struct TraceRun<'l> {
    steps: Vec<(Rc<Event<'l>>, Value<'l>)>,
    /// The start world, then the world after each step.
    worlds: Vec<Vec<Value<'l>>>,
}

impl<'l> TraceRun<'l> {
    fn value(&self, n: usize) -> Value<'l> {
        let world = |i: usize| Value::World(Rc::new(self.worlds[i].clone()));
        let steps = self
            .steps
            .iter()
            .take(n)
            .enumerate()
            .map(|(i, (e, r))| {
                Value::Step(Rc::new(Step {
                    event: Value::Event(e.clone()),
                    reply: r.clone(),
                    before: world(i),
                    after: world(i + 1),
                }))
            })
            .collect();
        let states = (0..self.worlds.len().min(n + 1)).map(world).collect();
        Value::Trace(Rc::new(Trace { steps, states }))
    }
}

enum Verdict {
    Holds,
    False,
    Broken(Diag),
}

fn verdict<'l>(ctx: &Ctx<'l>, law: &'l Law, run: &TraceRun<'l>, n: usize, others: &[Value<'l>]) -> Verdict {
    let mut m = match ctx.machine() {
        Ok(m) => m,
        Err(d) => return Verdict::Broken(d),
    };
    let mut env = Env::default();
    let mut others = others.iter();
    for (i, p) in law.params.iter().enumerate() {
        if i == law.trace {
            env.push(&p.name, run.value(n));
        } else if let Some(v) = others.next() {
            env.push(&p.name, v.clone());
        }
    }
    match m.eval(&mut env, &law.body) {
        Ok(Value::Bool(true)) => Verdict::Holds,
        Ok(Value::Bool(false)) => Verdict::False,
        Ok(_) => Verdict::Broken(Diag::error("E-TYPE-MISMATCH", law.body.span, "a law is a Bool")),
        Err(super::machine::Halt::Fail(f)) => Verdict::Broken(
            Diag::new(Severity::Failure, "F-LAW", format!("the law failed with {} instead of giving a Bool", f.name))
                .with_span(law.body.span),
        ),
        Err(h) => Verdict::Broken(halt_diag(h, ctx.lir)),
    }
}

pub(crate) fn run_law(ctx: &Ctx<'_>, l: LawId) -> (Status, Option<Diag>) {
    let lir = ctx.lir;
    let cfg = ctx.cfg;
    let law = lir.law(l);
    let merged = first_failure(cfg.traces, cfg.jobs, |i| {
        let ctx = Ctx::new(lir, cfg);
        let failure = match one_trace(&ctx, law, i) {
            Err(d) => Some(d),
            Ok((run, others)) => match verdict(&ctx, law, &run, run.steps.len(), &others) {
                Verdict::Holds => None,
                Verdict::Broken(d) => Some(d),
                Verdict::False => Some(
                    Diag::new(Severity::Failure, "F-LAW", format!("law {} is false on trace {i}", law.info.name))
                        .with_span(law.body.span)
                        .with_note(format!(
                            "trace {i}, seed {}; `spec-lang test` shrinks it",
                            seed_for(cfg.seed, &law.info.qualified, u64::from(i))
                        )),
                ),
            },
        };
        RunResult { failure }
    });
    match merged.first {
        None => (Status::Pass, None),
        Some((_, d)) => (Status::Fail, Some(d)),
    }
}

fn one_trace<'l>(ctx: &Ctx<'l>, law: &'l Law, i: u32) -> Result<(TraceRun<'l>, Vec<Value<'l>>), Diag> {
    let seed = seed_for(ctx.cfg.seed, &law.info.qualified, u64::from(i));
    let mut rng = Rng::new(seed);
    let start = if i % 2 == 1 { lawful_world(ctx, &mut rng)? } else { None };
    let run = generate(ctx, &mut rng, &law.traced, &law.start_rules, start)?;
    let others = other_params(ctx, law, &run, &mut rng)?;
    Ok((run, others))
}

/// A world drawn from the states' declared types on which every law holds
/// at the start; `None` when no try met them all.
fn lawful_world<'l>(ctx: &Ctx<'l>, rng: &mut Rng) -> Result<Option<Vec<Value<'l>>>, Diag> {
    let lir = ctx.lir;
    let mut m = ctx.machine()?;
    for _ in 0..WORLD_TRIES {
        let mut states = Vec::new();
        for s in &lir.states {
            states.push(value(&mut m, rng, &Pools::default(), &s.check).map_err(|e| e.into_diag(s.info.span))?);
        }
        let run = TraceRun { steps: Vec::new(), worlds: vec![states.clone()] };
        if is_lawful(ctx, &run, rng)? {
            return Ok(Some(states));
        }
    }
    Ok(None)
}

fn is_lawful<'l>(ctx: &Ctx<'l>, run: &TraceRun<'l>, rng: &mut Rng) -> Result<bool, Diag> {
    for law in &ctx.lir.laws {
        let others = other_params(ctx, law, run, rng)?;
        if !matches!(verdict(ctx, law, run, 0, &others), Verdict::Holds) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Parameters besides the trace, drawn after it with pools of its worlds.
fn other_params<'l>(ctx: &Ctx<'l>, law: &'l Law, run: &TraceRun<'l>, rng: &mut Rng) -> Result<Vec<Value<'l>>, Diag> {
    let pools = Pools::from_worlds(ctx.lir, run.worlds.iter());
    let mut m = ctx.machine()?;
    let mut out = Vec::new();
    for (i, p) in law.params.iter().enumerate() {
        if i != law.trace {
            out.push(value(&mut m, rng, &pools, &p.check).map_err(|e| e.into_diag(law.info.span))?);
        }
    }
    Ok(out)
}

fn random_event<'l>(
    ctx: &Ctx<'l>,
    m: &mut Machine<'l>,
    rng: &mut Rng,
    rules: &[RuleId],
) -> Result<Rc<Event<'l>>, Diag> {
    let Some(&rule) = rng.pick(rules) else {
        return Err(Diag::new(Severity::Error, "E-GEN", "a trace with no rules"));
    };
    let pools = Pools::from_worlds(ctx.lir, std::iter::once(&m.world));
    let mut args = Vec::new();
    for p in &ctx.lir.rule(rule).params {
        args.push(value(m, rng, &pools, &p.check).map_err(|e| e.into_diag(ctx.lir.rule(rule).info.span))?);
    }
    Ok(Rc::new(Event { rule, args }))
}

/// Call a rule from outside; its reply as `Ok(v)` or `Err(failure)`.
fn call<'l>(ctx: &Ctx<'l>, m: &mut Machine<'l>, e: &Event<'l>) -> Result<Value<'l>, Diag> {
    let lir = ctx.lir;
    let (tag, v) = match m.step(e.rule, e.args.clone()) {
        Ok(Ok(v)) => (spec_lang_ir::OK_TAG, v),
        Ok(Err(f)) => (spec_lang_ir::ERR_TAG, Value::Failure(f)),
        Err(h) => {
            let call = super::show::show(lir, &Value::Event(Rc::new(e.clone())));
            return Err(halt_diag(h, lir).with_note(format!("while running {call}")));
        }
    };
    Ok(Value::Variant(lir.result_type, tag, Rc::new(vec![v])))
}

fn generate<'l>(
    ctx: &Ctx<'l>,
    rng: &mut Rng,
    traced: &[RuleId],
    prefix: &[RuleId],
    start: Option<Vec<Value<'l>>>,
) -> Result<TraceRun<'l>, Diag> {
    let mut m = ctx.machine()?;
    let n_prefix = match start {
        Some(w) => {
            m.world = w;
            0
        }
        None => rng.below(u64::from(ctx.cfg.prefix) + 1),
    };
    for _ in 0..n_prefix {
        let e = random_event(ctx, &mut m, rng, prefix)?;
        call(ctx, &mut m, &e)?;
    }
    let mut run = TraceRun { steps: Vec::new(), worlds: vec![m.world.clone()] };
    for _ in 0..ctx.cfg.steps {
        let e = random_event(ctx, &mut m, rng, traced)?;
        let r = call(ctx, &mut m, &e)?;
        run.steps.push((e, r));
        run.worlds.push(m.world.clone());
    }
    Ok(run)
}
