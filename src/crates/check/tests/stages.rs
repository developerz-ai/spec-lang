//! The check and lint stages on small programs.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use spec_lang_ir::{Program, load_sources};

fn program(src: &str) -> Program {
    match load_sources(vec![("m".into(), "m.slg".into(), src.to_string())]) {
        Ok(p) => p,
        Err(f) => panic!("{}", f.render()),
    }
}

#[test]
fn a_pub_rule_lists_every_effect_of_its_body() {
    let p = program(
        "state n: Int = 0;\nstate log: List[Int];\n@why(\"x\")\n@uses(n.write)\npub rule r() = { n = n + 1; log.push(n); }",
    );
    let errors = spec_lang_check::check(&p);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "E-EFFECT-UNDECLARED");
    assert_eq!(errors[0].expected.as_deref(), Some("@uses(log.write, n.write)"));
    assert_eq!(errors[0].decl.as_deref(), Some("m.r"));
}

#[test]
fn defaults_are_of_their_types() {
    let p = program("type Small = Int where self < 3;\nfn f(x: Small = 5) -> Int = x;");
    let errors = spec_lang_check::check(&p);
    assert_eq!(errors[0].code, "E-TYPE-MISMATCH");
}

#[test]
fn lint_finds_unused_names_missing_reasons_and_empty_tests() {
    let p = program(
        "state n: Int = 0;\nfn unused() -> Int = 1;\n@uses(n.write)\npub rule r() = { n = 1; }\ntest t = { r(); }",
    );
    let codes: Vec<&str> = spec_lang_check::lint(&p).iter().map(|d| d.code).collect();
    assert!(codes.contains(&"W-LINT-UNUSED-DECL"), "{codes:?}");
    assert!(codes.contains(&"W-LINT-WHY"), "{codes:?}");
    assert!(codes.contains(&"W-LINT-NO-EXPECT"), "{codes:?}");
}
