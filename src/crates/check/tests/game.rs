//! Effects of `foreign` declarations, `@uses` on a `fn`, and the `game`
//! vocabulary in the check and lint stages.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use spec_lang_ir::{Program, load_sources};

fn program(src: &str) -> Program {
    match load_sources(vec![("m".into(), "m.slg".into(), format!("use game;\n{src}"))]) {
        Ok(p) => p,
        Err(f) => panic!("{}", f.render()),
    }
}

const KEYS: &str = "@uses(keys.read)\nfn key_down(key: Int) -> Bool = foreign c \"raylib#IsKeyDown\";\n";
const DRAW: &str = "@uses(screen.write)\nrule draw(x: Int) = foreign c \"raylib#DrawText\";\n";

fn codes(p: &Program) -> Vec<&'static str> {
    spec_lang_check::check(p).iter().map(|d| d.code).collect()
}

#[test]
fn a_world_effect_passes_to_whatever_calls_a_foreign_declaration() {
    let p = program(&format!(
        "{KEYS}{DRAW}@why(\"x\")\npub rule present() = {{ draw(1); }}\npub fn jump() -> Bool = key_down(32);\n"
    ));
    let errors = spec_lang_check::check(&p);
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert_eq!(errors[0].decl.as_deref(), Some("m.jump"));
    assert_eq!(errors[0].expected.as_deref(), Some("@uses(keys.read)"));
    assert_eq!(errors[1].decl.as_deref(), Some("m.present"));
    assert_eq!(errors[1].expected.as_deref(), Some("@uses(screen.write)"));
    assert!(errors.iter().all(|d| d.code == "E-EFFECT-UNDECLARED"));
}

#[test]
fn uses_on_a_fn_is_checked_like_on_a_rule() {
    let ok = program(&format!("{KEYS}@uses(keys.read)\npub fn jump() -> Bool = key_down(32);\n"));
    assert!(codes(&ok).is_empty());
    // Written `@uses` lists state reads too.
    let p = program(&format!("{KEYS}state n: Int = 0;\n@uses(keys.read)\nfn jump() -> Bool = key_down(n);\n"));
    let errors = spec_lang_check::check(&p);
    assert_eq!(errors[0].code, "E-EFFECT-UNDECLARED");
    assert_eq!(errors[0].expected.as_deref(), Some("@uses(keys.read, n.read)"));
    // A private fn without `@uses` is inferred; a pub fn reading only state needs none.
    let p = program(&format!("{KEYS}state n: Int = 0;\nfn jump() -> Bool = key_down(32);\npub fn now() -> Int = n;\n"));
    assert!(codes(&p).is_empty());
}

#[test]
fn a_fn_cannot_write_the_world_and_a_foreign_one_states_only_world_effects() {
    let p = program("@uses(screen.write)\nfn draw(x: Int) -> Int = foreign c \"raylib#DrawText\";\n");
    assert_eq!(codes(&p), ["E-EFFECT-KIND"]);
    let p = program("state n: Int = 0;\n@uses(n.write)\nrule set(x: Int) = foreign c \"lib#set\";\n");
    assert_eq!(codes(&p), ["E-UNSUPPORTED"]);
}

#[test]
fn lint_counts_use_game_as_used_and_unused_uses_on_a_fn() {
    let used = program(&format!("{KEYS}@uses(keys.read)\npub fn jump() -> Bool = key_down(32);\n"));
    assert!(spec_lang_check::lint(&used).is_empty(), "{:?}", spec_lang_check::lint(&used));
    let unused = program("pub fn one() -> Int = 1;\n");
    let codes: Vec<&str> = spec_lang_check::lint(&unused).iter().map(|d| d.code).collect();
    assert_eq!(codes, ["W-LINT-UNUSED-USE"]);
    let extra = program("@uses(keys.read)\npub fn one() -> Int = 1;\n");
    let codes: Vec<&str> = spec_lang_check::lint(&extra).iter().map(|d| d.code).collect();
    assert_eq!(codes, ["W-LINT-EFFECT-UNUSED"]);
}

#[test]
fn a_game_app_makes_its_entries_used() {
    let p = program(
        "state n: Int = 0;\nrule start() = { n = 0; }\n@on(tick(60Hz))\nrule step() = { n = n + 1; }\n\
         fn pad() -> Int = 0;\nrule frame() = { return; }\nfn dump() -> List[Text] = [];\nfn report() -> Text = \"\";\n\
         app G(start: start, tick: step, input: pad, frame: frame, dump: dump, report: report);\n",
    );
    let refs = spec_lang_check::refs::decl_refs(&p);
    let app = refs.get(&Some(spec_lang_ir::DeclRef::App(spec_lang_ir::AppId(0)))).unwrap();
    assert_eq!(app.len(), 6);
    assert!(spec_lang_check::lint(&p).is_empty(), "{:?}", spec_lang_check::lint(&p));
}
