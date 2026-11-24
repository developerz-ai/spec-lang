//! Libraries load from `libs/<name>/` on the search path, and everything a
//! program takes from one comes from its files: a library the toolchain has
//! never seen works the same as `cli` or `game`.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use spec_lang_ir::*;
use std::path::{Path, PathBuf};

/// A fresh library root holding the given `<lib>/<file>` texts.
fn root(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("slg-libs-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (path, text) in files {
        let p = dir.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    dir
}

fn load(libs: &Path, src: &str) -> Result<Program, Vec<IrError>> {
    let path = LibPath { dirs: vec![libs.to_path_buf()] };
    load_sources_with(vec![("m".into(), "m.slg".into(), src.to_string())], &path).map_err(|f| f.errors)
}

const BELL_VOCAB: &str = "@allows(on)\nnote ring(times: 1..=3);\n\
    @allows(app)\nnote tone(pitch: Pitch);\n\
    type Pitch = low | high(octave: Int);\n\
    @uses(bell.write)\nhandler bell {\n  @uses(bell.write)\n  rule ding(n: Int);\n  test count -> Int;\n}\n";
const BELL_PRETEND: &str = "handler PretendBell(rung: mut List[Int] = []) = {\n  bell.ding(n) -> rung.push(n);\n  bell.count -> rung.sum();\n}\n";

fn bell() -> PathBuf {
    root("bell", &[("bell/vocab.slg", BELL_VOCAB), ("bell/pretend.slg", BELL_PRETEND)])
}

#[test]
fn an_unknown_library_gives_its_notes_triggers_handlers_and_pretend() {
    let libs = bell();
    let src = "use bell;\n@on(ring(2))\nrule r() = { bell.ding(2); }\n\
        @tone(high(octave: 4))\napp A(rules: [r]);\n\
        test t = { r(); assert bell.count == 2; }\n";
    let p = load(&libs, src).unwrap();
    let lib = p.library("bell").unwrap();
    assert_eq!(lib.notes.iter().map(|n| n.form()).collect::<Vec<_>>(), ["ring(times: 1..=3)", "@tone(pitch: Pitch)"]);
    let h = p.handler(lib.handlers[0]);
    assert_eq!((h.name.as_str(), h.labels.as_slice()), ("bell", ["bell.write".to_string()].as_slice()));
    assert_eq!(h.pretend.as_ref().unwrap().arms.iter().flatten().count(), 2);
    assert_eq!(p.rules[0].triggers[0].to_string(), "ring(2)");
    assert_eq!(p.apps[0].notes[0].to_string(), "tone(high(4))");
    assert_eq!(p.world_labels().into_iter().collect::<Vec<_>>(), ["bell.write"]);
}

#[test]
fn a_library_use_is_checked_against_its_declarations() {
    let libs = bell();
    let code = |src: &str| load(&libs, src).err().map(|e| e[0].code).unwrap_or("loaded");
    assert_eq!(code("use bell;\n@on(ring(4))\nrule r() = { }\n"), "E-NOTE-ARG");
    assert_eq!(code("use bell;\n@tone(medium)\napp A(rules: []);\n"), "E-NOTE-ARG");
    assert_eq!(code("use bell;\n@tone(low)\nrule r() = { }\n"), "E-NOTE-PLACE");
    assert_eq!(code("use bell;\nfn f() -> Int = { bell.ding(1); return 1; }\n"), "E-EFFECT-KIND");
    assert_eq!(code("use bell;\nrule r() = { let n = bell.count; }\n"), "E-EFFECT-KIND");
    assert_eq!(code("use bells;\n"), "E-NAME-UNKNOWN");
}

#[test]
fn a_vocabulary_declares_only_what_a_library_may_add() {
    let libs = root("bad", &[("bad/vocab.slg", "rule r() = { }\n")]);
    assert_eq!(load(&libs, "use bad;\n").unwrap_err()[0].code, "E-LIB-FORM");
    let libs = root("bad2", &[("bad2/vocab.slg", "note n(x: Text);\n")]);
    assert_eq!(load(&libs, "use bad2;\n").unwrap_err()[0].code, "E-LIB-FORM");
}

#[test]
fn a_contract_is_a_module_named_after_its_library() {
    let p = load_sources(vec![(
        "m".into(),
        "m.slg".into(),
        "use raylib;\n@uses(keys.read)\npub fn space() -> Bool = raylib.is_key_down(32);\n".into(),
    )])
    .unwrap();
    let m = p.find_module("raylib").unwrap();
    assert_eq!(m.library.as_deref(), Some("raylib"));
    assert_eq!(p.library("raylib").unwrap().contract, Some(m.id));
    let f = p.fns.iter().find(|f| f.info.name == "is_key_down").unwrap();
    assert_eq!(f.foreign.as_ref().unwrap().symbol, "raylib#IsKeyDown");
    // The contract uses `game` for its effects, so loading raylib loaded game, and game loaded cli.
    let names: Vec<&str> = p.libraries.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["cli", "game", "raylib"]);
}

#[test]
fn the_search_path_finds_libs_beside_the_project_first() {
    let dir = root("walk", &[("libs/bell/vocab.slg", BELL_VOCAB), ("app/src/m.slg", "use bell;\n")]);
    let path = LibPath::for_project(&dir.join("app/src"));
    assert_eq!(path.find("bell").unwrap().canonicalize().unwrap(), dir.join("libs/bell").canonicalize().unwrap());
    assert!(path.find("cli").is_some(), "the builtin root comes last");
    assert!(load_project(&dir.join("app/src")).is_ok());
}
