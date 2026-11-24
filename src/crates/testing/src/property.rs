//! Property tests: parameters generated from their types, failures shrunk.

use crate::generate::{Pools, value};
use crate::parallel::{RunResult, first_failure};
use crate::report::Status;
use crate::rng::{Rng, seed_for};
use crate::scenario::{Ctx, Run};
use crate::shrink::candidates;
use spec_lang_diag::Diag;
use spec_lang_engine::Value;
use spec_lang_engine::display::show;
use spec_lang_ir::*;

/// Reruns allowed while shrinking one failure.
const SHRINK_BUDGET: u32 = 300;

pub(crate) fn run_property(ctx: &mut Ctx<'_>, t: TestId) -> (Status, Option<Diag>) {
    let prog = ctx.prog;
    let test = prog.test(t);
    let decl = prog.qualified(&test.info);
    let cfg = ctx.cfg;
    // Every run on a worker with its own world; merged by run number.
    let merged = first_failure(cfg.runs, cfg.jobs, |run| {
        let mut ctx = Ctx::new(prog, cfg);
        let failure = match args_of(&ctx, t, &decl, run) {
            Err(d) => Some((Status::Fail, d)),
            Ok(args) => match ctx.run_test(t, args) {
                Run::Pass(_) => None,
                Run::Blocked(d) => Some((Status::Blocked, d)),
                Run::Fail(d) => Some((Status::Fail, d)),
            },
        };
        RunResult { tally: 0, failure }
    });
    let Some((run, (status, d))) = merged.first else {
        return (Status::Pass, None);
    };
    if status != Status::Fail || d.code == "E-GEN" {
        return (status, Some(d));
    }
    // Generate that run's arguments again here and shrink them.
    let args = match args_of(ctx, t, &decl, run) {
        Ok(a) => a,
        Err(d) => return (Status::Fail, Some(d)),
    };
    let (args, d) = shrink(ctx, t, args, d);
    let shown: Vec<String> =
        test.params.iter().zip(&args).map(|(p, v)| format!("{}: {}", p.name, show(prog, v))).collect();
    let seed = seed_for(cfg.seed, &decl, u64::from(run));
    let d = d
        .with_note(format!("counterexample: {}({})", test.info.name, shown.join(", ")))
        .with_note(format!("run {run} of {}, seed {seed} (base seed {})", cfg.runs, cfg.seed));
    (Status::Fail, Some(d))
}

/// The arguments of run number `run`: a pure function of the seed, the
/// test's name and the number.
fn args_of(ctx: &Ctx<'_>, t: TestId, decl: &str, run: u32) -> Result<Vec<Value>, Diag> {
    let mut rng = Rng::new(seed_for(ctx.cfg.seed, decl, u64::from(run)));
    let mut it = ctx.interp()?;
    let mut args = Vec::new();
    for p in &ctx.prog.test(t).params {
        args.push(value(&mut it, &mut rng, &Pools::default(), &p.ty).map_err(|e| e.into_diag(p.span).in_decl(decl))?);
    }
    Ok(args)
}

fn shrink(ctx: &mut Ctx<'_>, t: TestId, mut args: Vec<Value>, mut d: Diag) -> (Vec<Value>, Diag) {
    let mut budget = SHRINK_BUDGET;
    let tys: Vec<Ty> = ctx.prog.test(t).params.iter().map(|p| p.ty.clone()).collect();
    'outer: loop {
        for i in 0..args.len() {
            for c in candidates(&args[i]) {
                if budget == 0 {
                    break 'outer;
                }
                budget -= 1;
                let Ok(mut it) = ctx.interp() else { break 'outer };
                if !matches!(it.conform(&c, &tys[i]), Ok(None)) {
                    continue;
                }
                let mut next = args.clone();
                next[i] = c;
                if let Run::Fail(nd) = ctx.run_test(t, next.clone())
                    && nd.code == d.code
                {
                    args = next;
                    d = nd;
                    continue 'outer;
                }
            }
        }
        break;
    }
    (args, d)
}
