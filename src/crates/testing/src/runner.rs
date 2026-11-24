//! Run every check of a program, in module order then declaration order.

use crate::config::Config;
use crate::law::run_law;
use crate::property::run_property;
use crate::report::{CheckResult, Evidence, Kind, Report, Status};
use crate::scenario::{CaseTarget, Ctx, Run, run_case};
use spec_lang_diag::Diag;
use spec_lang_ir::*;
use std::time::Instant;

pub fn run_all(prog: &Program, cfg: Config) -> Report {
    let start = Instant::now();
    let mut ctx = Ctx::new(prog, cfg);
    let mut checks = Vec::new();
    let mut modules: Vec<&Module> = prog.modules.iter().skip(1).collect();
    modules.sort_by(|a, b| a.name.cmp(&b.name));
    for m in modules {
        for d in &m.decls {
            match *d {
                DeclRef::Fn(f) => {
                    let decl = prog.qualified(&prog.func(f).info);
                    for (i, case) in prog.func(f).cases.iter().enumerate() {
                        checks.push(timed(
                            &m.name,
                            Kind::Case,
                            &decl,
                            prog.snippet(case.span),
                            Evidence::Example,
                            || outcome(run_case(&mut ctx, CaseTarget::Fn(f), case, &decl, format!("case[{i}]"))),
                        ));
                    }
                }
                DeclRef::Test(t) => {
                    let test = prog.test(t);
                    let decl = prog.qualified(&test.info);
                    if !test.cases.is_empty() {
                        for (i, case) in test.cases.iter().enumerate() {
                            checks.push(timed(
                                &m.name,
                                Kind::Case,
                                &decl,
                                prog.snippet(case.span),
                                Evidence::Example,
                                || outcome(run_case(&mut ctx, CaseTarget::Test(t), case, &decl, format!("case[{i}]"))),
                            ));
                        }
                    } else if !test.params.is_empty() {
                        let label = params_label(prog, &test.params);
                        checks.push(timed(&m.name, Kind::Property, &decl, &label, Evidence::Sampled(cfg.runs), || {
                            run_property(&mut ctx, t)
                        }));
                    } else {
                        checks.push(timed(&m.name, Kind::Test, &decl, "", Evidence::Example, || match ctx.end_of(t) {
                            Run::Pass(_) => (Status::Pass, None),
                            Run::Fail(d) => (Status::Fail, Some(d)),
                            Run::Blocked(d) => (Status::Blocked, Some(d)),
                        }));
                    }
                }
                DeclRef::Law(l) => {
                    let law = prog.law(l);
                    let decl = prog.qualified(&law.info);
                    let label = params_label(prog, &law.params);
                    let mut lawful = 0;
                    let mut check = timed(&m.name, Kind::Law, &decl, &label, Evidence::Example, || {
                        run_law(&mut ctx, l, &mut lawful)
                    });
                    if !check.blocked_by_foreign() {
                        check.evidence = Evidence::Traces { traces: cfg.traces, steps: cfg.steps, lawful };
                    }
                    checks.push(check);
                }
                _ => {}
            }
        }
    }
    Report { config: cfg, checks, micros: start.elapsed().as_micros() }
}

fn params_label(prog: &Program, params: &[Param]) -> String {
    let ps: Vec<String> =
        params.iter().map(|p| format!("{}: {}", p.name, spec_lang_engine::display::show_ty(prog, &p.ty))).collect();
    format!("({})", ps.join(", "))
}

fn outcome(r: Result<(), Diag>) -> (Status, Option<Diag>) {
    match r {
        Ok(()) => (Status::Pass, None),
        Err(d) if d.code == "F-BLOCKED" => (Status::Blocked, Some(d)),
        Err(d) => (Status::Fail, Some(d)),
    }
}

fn timed(
    module: &str,
    kind: Kind,
    decl: &str,
    label: &str,
    evidence: Evidence,
    f: impl FnOnce() -> (Status, Option<Diag>),
) -> CheckResult {
    let start = Instant::now();
    let (mut status, diag) = f();
    let mut evidence = evidence;
    // A check that reached a foreign body is blocked, whatever stage of it
    // reached the body (a case, a step, a trace, a generated run).
    if let Some(d) = diag.as_ref().filter(|d| d.code == "B-FOREIGN") {
        status = Status::Blocked;
        evidence = Evidence::Assumed(d.found.clone().unwrap_or_else(|| "foreign".to_string()));
    }
    CheckResult {
        module: module.to_string(),
        kind,
        decl: decl.to_string(),
        label: label.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" "),
        status,
        evidence,
        diag,
        micros: start.elapsed().as_micros(),
    }
}
