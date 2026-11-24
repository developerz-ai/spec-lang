//! Random traces: a start world reached by random calls, then random calls
//! of the traced rules, every argument generated from its parameter's type.

use crate::generate::{Pools, value};
use crate::rng::Rng;
use crate::scenario::{Ctx, halt_diag};
use spec_lang_diag::Diag;
use spec_lang_engine::{Event, Interp, Outcome, Step, Trace, Value, World};
use spec_lang_ir::*;
use std::rc::Rc;

/// One executed trace.
#[derive(Clone, Debug)]
pub(crate) struct TraceRun {
    /// A start world drawn from the types and laws, when the trace did not
    /// start from the initial world.
    pub(crate) start: Option<World>,
    /// Calls that built the start world (not part of `t`).
    pub(crate) prefix: Vec<(Rc<Event>, Value)>,
    /// The trace's steps with their replies.
    pub(crate) steps: Vec<(Rc<Event>, Value)>,
    /// The start world, then the world after each step.
    pub(crate) worlds: Vec<World>,
}

impl TraceRun {
    /// `t` as the law sees it, cut after `n` steps.
    pub(crate) fn value(&self, n: usize) -> Value {
        let mut steps = Vec::new();
        for (i, (e, r)) in self.steps.iter().take(n).enumerate() {
            steps.push(Value::Step(Rc::new(Step {
                event: Value::Event(e.clone()),
                reply: r.clone(),
                before: Value::World(Rc::new(self.worlds[i].clone())),
                after: Value::World(Rc::new(self.worlds[i + 1].clone())),
            })));
        }
        let states = self.worlds.iter().take(n + 1).map(|w| Value::World(Rc::new(w.clone()))).collect();
        Value::Trace(Rc::new(Trace { steps, states }))
    }
}

/// A step's reply: `Ok(v)`, or `Err(e)` holding the failure (card, Traces).
fn reply(prog: &Program, o: Outcome) -> Value {
    let (tag, v) = match o {
        Outcome::Ok(v) => (OK_TAG, v),
        Outcome::Refused(f) => (ERR_TAG, Value::Failure(f)),
    };
    Value::Variant(prog.result_type, tag, Rc::new(vec![v]))
}

fn call(ctx: &Ctx<'_>, it: &mut Interp<'_>, e: &Event) -> Result<Value, Diag> {
    let span = ctx.prog.rule(e.rule).info.name_span;
    let call = spec_lang_engine::display::call(ctx.prog, e);
    it.step(e.rule, e.args.clone())
        .map(|o| reply(ctx.prog, o))
        .map_err(|h| halt_diag(ctx.prog, h, Some(span)).with_note(format!("while running {call}")))
}

/// The rules whose calls may build a start world: every rule of the modules
/// that own the traced rules, except one that can reach a `foreign` body
/// (only a backend runs it; a traced rule that reaches one still blocks).
pub(crate) fn prefix_rules(prog: &Program, traced: &[RuleId]) -> Vec<RuleId> {
    let modules: Vec<ModuleId> = traced.iter().map(|r| prog.rule(*r).info.module).collect();
    (0..prog.rules.len())
        .map(|i| RuleId(u32::try_from(i).unwrap_or(0)))
        .filter(|r| modules.contains(&prog.rule(*r).info.module) && !reaches_foreign(prog, *r))
        .collect()
}

/// Whether a call of `r` can reach a `foreign` fn or rule through its guards
/// and body.
fn reaches_foreign(prog: &Program, r: RuleId) -> bool {
    let mut seen = vec![Callee::Rule(r)];
    let mut todo = vec![Callee::Rule(r)];
    while let Some(c) = todo.pop() {
        let exprs: Vec<&Expr> = match c {
            Callee::Fn(f) if prog.func(f).foreign.is_some() => return true,
            Callee::Rule(x) if prog.rule(x).foreign.is_some() => return true,
            Callee::Fn(f) => vec![&prog.func(f).body],
            Callee::Rule(x) => prog.rule(x).guards.iter().map(|g| &g.cond).chain([&prog.rule(x).body]).collect(),
            _ => continue,
        };
        for e in exprs {
            spec_lang_ir::visit::walk_expr(prog, e, &mut |x| {
                if let ExprKind::Call { callee: c @ (Callee::Fn(_) | Callee::Rule(_)), .. } = &x.kind
                    && !seen.contains(c)
                {
                    seen.push(*c);
                    todo.push(*c);
                }
            });
        }
    }
    false
}

fn random_event(ctx: &Ctx<'_>, it: &mut Interp<'_>, rng: &mut Rng, rules: &[RuleId]) -> Result<Rc<Event>, Diag> {
    let Some(&rule) = rng.pick(rules) else {
        return Err(Diag::new(spec_lang_diag::Severity::Error, "E-GEN", "a trace with no rules"));
    };
    let pools = Pools::from_worlds(ctx.prog, std::iter::once(&it.world));
    let mut args = Vec::new();
    for p in &ctx.prog.rule(rule).params {
        args.push(value(it, rng, &pools, &p.ty).map_err(|e| e.into_diag(p.span))?);
    }
    Ok(Rc::new(Event { rule, args }))
}

/// Generate and run one trace: from `start` if given, else from the initial
/// world and up to `prefix` random calls.
pub(crate) fn generate(
    ctx: &Ctx<'_>,
    rng: &mut Rng,
    traced: &[RuleId],
    prefix: &[RuleId],
    start: Option<World>,
) -> Result<TraceRun, Diag> {
    let mut it = ctx.interp()?;
    let mut run = TraceRun { start: start.clone(), prefix: Vec::new(), steps: Vec::new(), worlds: Vec::new() };
    let n_prefix = match start {
        Some(w) => {
            it.world = w;
            0
        }
        None => rng.below(u64::from(ctx.cfg.prefix) + 1),
    };
    for _ in 0..n_prefix {
        let e = random_event(ctx, &mut it, rng, prefix)?;
        let r = call(ctx, &mut it, &e)?;
        run.prefix.push((e, r));
    }
    run.worlds.push(it.world.clone());
    for _ in 0..ctx.cfg.steps {
        let e = random_event(ctx, &mut it, rng, traced)?;
        let r = call(ctx, &mut it, &e)?;
        run.steps.push((e, r));
        run.worlds.push(it.world.clone());
    }
    Ok(run)
}

/// Run fixed calls again from the same start: used while shrinking.
pub(crate) fn replay(
    ctx: &Ctx<'_>,
    start: &Option<World>,
    prefix: &[Rc<Event>],
    steps: &[Rc<Event>],
) -> Result<TraceRun, Diag> {
    let mut it = ctx.interp()?;
    if let Some(w) = start {
        it.world = w.clone();
    }
    let mut run = TraceRun { start: start.clone(), prefix: Vec::new(), steps: Vec::new(), worlds: Vec::new() };
    for e in prefix {
        let r = call(ctx, &mut it, e)?;
        run.prefix.push((e.clone(), r));
    }
    run.worlds.push(it.world.clone());
    for e in steps {
        let r = call(ctx, &mut it, e)?;
        run.steps.push((e.clone(), r));
        run.worlds.push(it.world.clone());
    }
    Ok(run)
}
