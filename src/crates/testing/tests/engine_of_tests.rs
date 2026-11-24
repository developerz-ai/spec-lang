//! The test engine finds, shrinks and reports.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use spec_lang_ir::{Program, load_sources};
use spec_lang_testing::{Config, Kind, Status, run_all};

fn program(src: &str) -> Program {
    match load_sources(vec![("m".into(), "m.slg".into(), src.to_string())]) {
        Ok(p) => p,
        Err(f) => panic!("{}", f.render()),
    }
}

fn small() -> Config {
    Config { traces: 50, steps: 10, runs: 50, ..Config::default() }
}

#[test]
fn a_false_law_gives_a_short_counterexample_as_a_test() {
    let p = program(
        "state n: Int = 0;\nrule up(k: Int) = { n = n + k; }\n\
         law stays_small(t: Trace[up]) = t.states.all(w => w.n < 5);",
    );
    let r = run_all(&p, small());
    let law = r.checks.iter().find(|c| c.kind == Kind::Law).unwrap();
    assert_eq!(law.status, Status::Fail);
    let d = law.diag.as_ref().unwrap();
    assert_eq!(d.code, "F-LAW");
    let notes = d.notes.join("\n");
    assert!(notes.contains("test stays_small_counterexample = {"), "{notes}");
    assert!(notes.contains("m.up(5)") || notes.contains("m.up("), "{notes}");
    assert!(notes.contains("seed"), "{notes}");
}

#[test]
fn a_property_failure_is_shrunk() {
    let p = program("test small(n: Int) = { assert n < 7; }");
    let r = run_all(&p, small());
    let d = r.checks[0].diag.as_ref().unwrap();
    assert!(d.notes.iter().any(|n| n.contains("small(n: 7)")), "{:?}", d.notes);
}

#[test]
fn a_child_of_a_failing_test_is_blocked_and_runs_are_deterministic() {
    let p = program(
        "state n: Int = 0;\nrule up() = { n = n + 1; }\ntest a = { up() -> 1; assert n == 2; }\n@given(a)\ntest b = { up(); }",
    );
    let r = run_all(&p, small());
    let status: Vec<Status> = r.checks.iter().map(|c| c.status).collect();
    assert_eq!(status, [Status::Fail, Status::Blocked]);
    let again = run_all(&p, small());
    assert_eq!(format!("{:?}", again.checks[0].diag), format!("{:?}", r.checks[0].diag));
}
