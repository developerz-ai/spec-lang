//! When a law breaks from a random lawful world, the world may be one no
//! rule can reach: then the program lacks a state law (card, How it runs 2:
//! "offers the missing law"). This module proposes one.
//!
//! Candidates come from a small catalogue of shapes; one is offered only if
//! it is false on the counterexample's start world and true on every world
//! a sample of reached traces visited. Sampling, not proof: the offer says so.

use crate::rng::{Rng, seed_for};
use crate::scenario::Ctx;
use crate::trace::{generate, prefix_rules};
use spec_lang_diag::{Diag, Edit};
use spec_lang_engine::{Value, World};
use spec_lang_ir::*;
use spec_lang_syntax::Span;

/// Reached traces sampled to test a candidate law.
const SAMPLE: u64 = 40;

/// One candidate: its source and its test on a world.
struct Candidate {
    name: String,
    body: String,
    holds: Box<dyn Fn(&World) -> bool>,
}

/// `w.m.keys.all(k => w.m[k].f == k)` for every map state whose values are
/// records with a field of the key's type.
fn key_fields(prog: &Program) -> Vec<Candidate> {
    let mut out = Vec::new();
    for (i, s) in prog.states.iter().enumerate() {
        let Ty::Map(k, v) = &s.ty else { continue };
        let Ty::Named(t) = &**v else { continue };
        let TypeDef::Record(fields) = &prog.ty(*t).def else { continue };
        for (fi, f) in fields.iter().enumerate() {
            if f.ty != **k {
                continue;
            }
            let (m, field) = (s.info.name.clone(), f.name.clone());
            out.push(Candidate {
                name: format!("{m}_under_their_{field}"),
                body: format!("t.states.all(w => w.{m}.keys.all(k => w.{m}[k].{field} == k))"),
                holds: Box::new(move |w: &World| match w.states.get(i) {
                    Some(Value::Map(map)) => map.iter().all(|(key, val)| match val {
                        Value::Record(_, xs) => xs.get(fi) == Some(key),
                        _ => true,
                    }),
                    _ => true,
                }),
            });
        }
    }
    out
}

/// Worlds visited by reached traces of the same rules.
fn reached_worlds(ctx: &Ctx<'_>, law: &LawDecl, traced: &[RuleId]) -> Vec<World> {
    let prefix = prefix_rules(ctx.prog, traced);
    let name = format!("{}#missing", ctx.prog.qualified(&law.info));
    let mut out = Vec::new();
    for i in 0..SAMPLE {
        let mut rng = Rng::new(seed_for(ctx.cfg.seed, &name, i));
        if let Ok(run) = generate(ctx, &mut rng, traced, &prefix, None) {
            out.extend(run.worlds);
        }
    }
    out
}

/// Add the offer to a lawful-start counterexample: a candidate law that rules
/// the start world out, or the two answers when none fits.
pub(crate) fn offer(ctx: &Ctx<'_>, law: &LawDecl, start: &World, d: Diag) -> Diag {
    let prog = ctx.prog;
    let Some((ti, traced)) = law.trace() else { return d };
    let trace_param = prog.snippet(law.params[ti].span).to_string();
    let worlds = reached_worlds(ctx, law, traced);
    let fit = key_fields(prog).into_iter().find(|c| !(c.holds)(start) && worlds.iter().all(|w| (c.holds)(w)));
    let end = Span::new(law.info.span.file, law.info.span.hi, law.info.span.hi);
    let bug = "a rule is wrong: change it so the law holds from this world too";
    match fit {
        Some(c) => {
            let text = format!("\n\n@why(\"...\")\nlaw {}({trace_param}) =\n  {};", c.name, c.body);
            d.with_note(format!(
                "missing law? `{}` is false on this start world and held on all {} worlds of {SAMPLE} reached traces (sampled, not proven)",
                c.body,
                worlds.len()
            ))
            .with_alternative("no rule can reach this world: say so with a state law", vec![Edit { span: end, text }])
            .with_alternative(bug, Vec::new())
        }
        None => d
            .with_note("no candidate state law from the catalogue excludes this start world")
            .with_alternative(
                "no rule can reach this world: add a law `t.states.all(w => ...)` that excludes it",
                Vec::new(),
            )
            .with_alternative(bug, Vec::new()),
    }
}
