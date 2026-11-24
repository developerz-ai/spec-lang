//! The judge on the two POCs: the LIR evaluator reports what the engine
//! reports, check by check, with the same seed and counts. The platformer
//! runs with fewer traces here to keep a debug test short; `src/bin/corpus`
//! runs every program with the default counts.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use spec_lang_lir::eval;
use spec_lang_testing::{Config, Kind, Status, run_all};
use std::path::Path;

/// The engine's checks as `(name, status)`, named as the LIR names them.
fn engine(prog: &spec_lang_ir::Program, cfg: Config) -> Vec<(String, &'static str)> {
    let report = run_all(prog, cfg);
    let mut out = Vec::new();
    let mut row = 0;
    let mut last = String::new();
    for c in &report.checks {
        let name = if c.kind == Kind::Case {
            row = if c.decl == last { row + 1 } else { 0 };
            last = c.decl.clone();
            format!("{} case[{row}]", c.decl)
        } else {
            last.clear();
            c.decl.clone()
        };
        let status = match c.status {
            Status::Pass => "pass",
            Status::Fail => "fail",
            Status::Blocked => "blocked",
        };
        out.push((name, status));
    }
    out
}

fn agree(project: &str, traces: u32) -> Vec<(String, &'static str)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").join(project);
    let prog = spec_lang_ir::load_project(&dir).unwrap_or_else(|f| panic!("{}", f.render()));
    assert!(spec_lang_check::check(&prog).is_empty(), "{project} does not pass the check stage");
    let lir = spec_lang_lir::lower(&prog).unwrap_or_else(|ds| panic!("{ds:?}"));
    let cfg = Config { traces, ..Config::default() };
    let theirs = engine(&prog, cfg);
    let lcfg =
        eval::Config { seed: cfg.seed, runs: cfg.runs, traces, steps: cfg.steps, prefix: cfg.prefix, jobs: cfg.jobs };
    let ours: Vec<(String, &'static str)> =
        eval::run_all(&lir, lcfg).into_iter().map(|c| (c.name, c.status.as_str())).collect();
    assert_eq!(ours, theirs, "{project}: the LIR evaluator and the engine disagree");
    ours
}

#[test]
fn the_todo_app_agrees_with_the_engine() {
    let checks = agree("pocs/todo-app/src", 200);
    assert!(checks.iter().all(|(_, s)| *s == "pass"));
    assert_eq!(checks.len(), 35);
}

#[test]
fn the_platformer_agrees_with_the_engine() {
    let checks = agree("pocs/platformer/src", 20);
    let blocked: Vec<&str> = checks.iter().filter(|(_, s)| *s == "blocked").map(|(n, _)| n.as_str()).collect();
    assert_eq!(blocked, ["hop.tests.edge.touch_matches_raylib"]);
    assert!(checks.iter().all(|(_, s)| *s != "fail"));
}
