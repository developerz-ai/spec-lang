//! A check that reaches a `foreign` body is blocked with `B-FOREIGN` and
//! evidence `assumed`; checks that never reach one are unaffected.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use spec_lang_ir::{Program, load_sources};
use spec_lang_testing::{CheckResult, Config, Evidence, Report, Status, run_all};

fn program(src: &str) -> Program {
    let text = format!("use game;\nfn native(a: Int) -> Bool = foreign c \"lib#native\";\n{src}");
    match load_sources(vec![("m".into(), "m.slg".into(), text)]) {
        Ok(p) => p,
        Err(f) => panic!("{}", f.render()),
    }
}

fn run(src: &str) -> Report {
    run_all(&program(src), Config { traces: 20, steps: 10, runs: 20, ..Config::default() })
}

fn check<'r>(r: &'r Report, decl: &str) -> &'r CheckResult {
    r.checks.iter().find(|c| c.decl == decl).unwrap()
}

fn assumed() -> Evidence {
    Evidence::Assumed("foreign c \"lib#native\"".into())
}

#[test]
fn a_property_that_reaches_a_foreign_fn_is_blocked_and_the_run_stays_green() {
    let r = run("fn mine(a: Int) -> Bool = a > 0;\ntest same(a: Int) = { assert mine(a) == native(a); }\n");
    let c = check(&r, "m.same");
    assert_eq!(c.status, Status::Blocked);
    assert_eq!(c.evidence, assumed());
    assert_eq!(c.evidence.to_string(), "assumed (foreign c \"lib#native\": runs only on a backend)");
    let d = c.diag.as_ref().unwrap();
    assert_eq!(d.code, "B-FOREIGN");
    assert!(d.message.contains("m.native"), "{}", d.message);
    assert_eq!(d.decl.as_deref(), Some("m.same"));
    assert!(r.green());
    assert!(r.summary().starts_with("1 checks: 0 passed, 0 failed, 1 blocked"), "{}", r.summary());
}

#[test]
fn cases_scenarios_and_laws_that_reach_one_are_blocked_and_the_others_pass() {
    let r = run("@cases(1 -> true)\nfn twice(a: Int) -> Bool = native(a);\n\
         state n: Int = 0;\n@uses(n.write)\nrule up() = { n = n + 1; }\nrule poke() = { if native(n) { n = 0; } }\n\
         law grows(t: Trace[up]) = t.steps.all(it.after.n > it.before.n);\n\
         law pokes(t: Trace[poke]) = t.states.all(w => w.n >= 0);\n\
         test start = { up(); poke(); }\n@given(start)\ntest child = { assert n == 1; }\n\
         test plain = { up(); assert n == 1; }\n");
    for blocked in ["m.twice", "m.pokes", "m.start", "m.child"] {
        let c = check(&r, blocked);
        assert_eq!((c.decl.as_str(), c.status), (blocked, Status::Blocked));
        assert_eq!(c.diag.as_ref().unwrap().code, "B-FOREIGN");
        assert_eq!(c.evidence, assumed());
    }
    assert!(check(&r, "m.child").diag.as_ref().unwrap().message.contains("blocked by m.start"));
    assert_eq!(check(&r, "m.grows").status, Status::Pass);
    assert!(matches!(check(&r, "m.grows").evidence, Evidence::Traces { .. }));
    assert_eq!(check(&r, "m.plain").status, Status::Pass);
    assert!(r.green());
}

#[test]
fn a_hole_still_blocks_and_is_not_green() {
    let r = run("fn later() -> Int = ?later;\ntest t = { assert later() == 1; }\n");
    let c = check(&r, "m.t");
    assert_eq!(c.status, Status::Blocked);
    assert_eq!(c.diag.as_ref().unwrap().code, "F-BLOCKED");
    assert!(!r.green());
}

#[test]
fn a_failure_next_to_a_blocked_check_is_still_red() {
    let r = run("test bad = { assert 1 == 2; }\ntest edge = { assert native(1); }\n");
    assert_eq!(check(&r, "m.edge").status, Status::Blocked);
    assert_eq!(check(&r, "m.bad").status, Status::Fail);
    assert!(!r.green());
}
