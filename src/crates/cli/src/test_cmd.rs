//! `spec-lang test`: parse, check, lint, test; each stage reported apart.
//! A stage with errors stops the pipeline; warnings do not.

use crate::args::{TestArgs, parse_test_args};
use crate::output::Out;
use spec_lang_diag::Diag;
use spec_lang_ir::LoadFailure;
use spec_lang_testing::{Config, run_all};
use std::path::Path;

pub fn run(args: &[String]) -> u8 {
    let a = match parse_test_args(args) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let mut out = Out::new(a.json);
    match pipeline(&a, &mut out) {
        Ok(green) => u8::from(!green),
        Err(()) => 1,
    }
}

fn pipeline(a: &TestArgs, out: &mut Out) -> Result<bool, ()> {
    let prog = match spec_lang_ir::load_project(Path::new(&a.project)) {
        Ok(p) => p,
        Err(f) => {
            return {
                load_failed(&f, out);
                Err(())
            };
        }
    };
    let files = prog.modules.len() - 1;
    out.stage("parse", "ok", &format!("{files} files"));
    let errors = spec_lang_check::check(&prog);
    if !errors.is_empty() {
        out.stage("check", "error", &format!("{} errors", errors.len()));
        out.diags("check", &errors, &prog.sources);
        return Err(());
    }
    out.stage("check", "ok", "names, notes, guards, arity, state ownership, effects, defaults");
    let warnings = spec_lang_check::lint(&prog);
    let lint_status = if warnings.is_empty() {
        "ok"
    } else if a.deny_warnings {
        "error"
    } else {
        "warning"
    };
    out.stage("lint", lint_status, &format!("{} warnings", warnings.len()));
    out.diags("lint", &warnings, &prog.sources);
    let cfg =
        Config { seed: a.seed, runs: a.runs, traces: a.traces, steps: a.steps, jobs: a.jobs, ..Config::default() };
    let report = run_all(&prog, cfg);
    let status = if report.green() { "ok" } else { "failure" };
    out.stage("test", status, &report.settings());
    out.report(&report, &prog.sources);
    Ok(report.green() && (warnings.is_empty() || !a.deny_warnings))
}

/// The loader's errors: syntax ones are the parse stage, the rest the check stage.
fn load_failed(f: &LoadFailure, out: &mut Out) {
    let (parse, check): (Vec<Diag>, Vec<Diag>) =
        f.errors.iter().cloned().partition(|d| d.code.starts_with("E-PARSE") || d.code == "E-LOAD");
    if !parse.is_empty() {
        out.stage("parse", "error", &format!("{} errors", parse.len()));
        out.diags("parse", &parse, &f.sources);
        return;
    }
    out.stage("parse", "ok", "");
    out.stage("check", "error", &format!("{} errors", check.len()));
    out.diags("check", &check, &f.sources);
}
