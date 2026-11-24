//! The todo POC resolves completely.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use spec_lang_ir::*;
use std::path::Path;

fn todo() -> Program {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../pocs/todo-app/src");
    match load_project(&dir) {
        Ok(p) => p,
        Err(f) => panic!("{}", f.render()),
    }
}

#[test]
fn every_declaration_resolves() {
    let p = todo();
    let names: Vec<_> = p.modules.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "core",
            "apps.bun",
            "apps.cpp",
            "apps.ruby",
            "apps.rust",
            "todo.board",
            "todo.laws",
            "todo.model",
            "todo.render",
            "todo.tests.scenarios"
        ]
    );
    // The program grows; these are the floors the rest of this file leans on.
    assert_eq!(p.rules.len(), 8);
    assert!(p.laws.len() >= 6);
    assert!(p.tests.len() >= 10);
    assert_eq!(p.apps.len(), 4);
}

#[test]
fn guards_are_ordered_and_apps_carry_their_notes() {
    let p = todo();
    let done = p.rules.iter().find(|r| r.info.name == "done").unwrap();
    let errors: Vec<_> = done.guards.iter().map(|g| g.error.as_str()).collect();
    assert_eq!(errors, ["NoTask", "AlreadyDone"]);
    assert_eq!(done.triggers, [Trigger::Cli { command: "done".into() }]);
    let rust = p.apps.iter().find(|a| a.info.name == "TodoRust").unwrap();
    assert_eq!(rust.target.as_deref(), Some("rust"));
    assert_eq!(rust.bin.as_deref(), Some("todo"));
    assert_eq!(rust.store, Some(Store::File(".todo.json".into())));
    assert_eq!(rust.rules.len(), 8);
}

#[test]
fn rule_sets_flatten_into_traces() {
    let p = todo();
    let law = p.laws.iter().find(|l| l.info.name == "numbers_below_next").unwrap();
    let (_, rules) = law.trace().unwrap();
    assert_eq!(rules.len(), 6);
    assert!(!p.rule_set_uses.is_empty(), "Trace[Change] names a rule-set type");
}

#[test]
fn errors_carry_code_place_and_declaration() {
    let src = "rule r() = { x = 1; }\n".to_string();
    let f = load_sources(vec![("m".into(), "m.slg".into(), src)]).unwrap_err();
    let e = &f.errors[0];
    assert_eq!(e.code, "E-NAME-UNKNOWN");
    assert_eq!(e.decl.as_deref(), Some("m.r"));
    assert_eq!(e.at.as_deref(), Some("body.stmt[0]"));
}

fn one(src: &str) -> Result<Program, spec_lang_ir::LoadFailure> {
    load_sources(vec![("m".into(), "m.slg".into(), src.into())])
}

fn two(a: &str, b: &str) -> Result<Program, spec_lang_ir::LoadFailure> {
    load_sources(vec![("a".into(), "a.slg".into(), a.into()), ("b".into(), "b.slg".into(), b.into())])
}

fn code(r: Result<Program, spec_lang_ir::LoadFailure>) -> &'static str {
    match r {
        Ok(_) => "ok",
        Err(f) => f.errors[0].code,
    }
}

#[test]
fn another_modules_names_must_be_pub() {
    assert_eq!(code(two("fn f() -> Int = 1;", "use a.f;")), "E-NAME-PRIVATE");
    assert_eq!(code(two("pub fn f() -> Int = 1;", "use a.f;")), "ok");
    assert_eq!(code(two("fn f() -> Int = 1;", "use a;\nfn g() -> Int = a.f();")), "E-NAME-PRIVATE");
    assert_eq!(code(two("state n: Int = 0;\ntest t = {}", "use a;\n@given(a.t)\ntest u = { assert a.n == 0; }")), "ok");
}

#[test]
fn given_sets_only_its_own_modules_state() {
    let r = two("state n: Int = 0;", "use a;\n@given(a.n -> 1)\ntest u = {}");
    assert_eq!(code(r), "E-STATE-FOREIGN");
}

#[test]
fn a_parameter_may_take_a_free_name_but_not_call_it() {
    assert_eq!(code(one("fn f(all: Bool) -> Bool = all;")), "ok");
    assert_eq!(code(one("fn f(all: Bool) -> Bool = all(1);")), "E-NAME-SHADOW");
}

#[test]
fn replies_match_ok_and_err_and_fail_is_no_pattern() {
    let law = "rule r() = {}\nlaw l(t: Trace[r]) = t.steps.all(s => s.reply.is(Err(_)) or s.reply.is(Ok(_)));";
    assert_eq!(code(one(law)), "ok");
    let f = one("rule r() = {}\nlaw l(t: Trace[r]) = t.steps.all(s => s.reply.is(fail(_)));").unwrap_err();
    assert_eq!(f.errors[0].code, "E-NAME-KIND");
    assert_eq!(f.errors[0].edits[0].text, "Err(_)");
}
