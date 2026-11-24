//! The binary on a small `game` project: `test` is green with a check
//! blocked by a foreign body, `build` refuses the game app, `view` and
//! `parse` take the new forms.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const GAME: &str = "use game;\n\
    state n: Int = 0;\n\
    @why(\"x\")\n@uses(n.write, out.write)\npub rule start() = { n = 0; out.line(\"start\"); }\n\
    @why(\"x\")\n@on(tick(60Hz))\n@uses(n.write)\npub rule step(pad: Bool) = { if pad { n = n + 1; } }\n\
    @uses(keys.read)\nfn key_down(key: Int) -> Bool = foreign c \"raylib#IsKeyDown\";\n\
    @why(\"x\")\n@uses(keys.read)\npub fn read_pad() -> Bool = key_down(32);\n\
    @uses(screen.write)\nrule draw(x: Int) = foreign c \"raylib#DrawText\";\n\
    @why(\"x\")\n@uses(screen.write, n.read)\npub rule present() = { draw(n); }\n\
    pub fn dump() -> List[Text] = [\"{n}\"];\n\
    pub fn summary() -> Text = \"end {n}\";\n\
    test steps = { start(); step(true); assert n == 1; }\n\
    test reads = { assert read_pad(); }\n\
    @target(rust)\n@bin(\"g\")\n@window(64, 48, \"G\")\n\
    app G(start: start, tick: step, input: read_pad, frame: present, dump: dump, report: summary);\n";

fn project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("spec-lang-game-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("g.slg"), GAME).unwrap();
    dir
}

fn run(args: &[&str], dir: &Path) -> (Output, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_spec-lang")).args(args).arg(dir).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    (out, text)
}

#[test]
fn test_is_green_with_the_foreign_check_blocked() {
    let dir = project("test");
    let (out, text) = run(&["test", "--deny", "warnings"], &dir);
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert!(text.contains("2 checks: 1 passed, 0 failed, 1 blocked"), "{text}");
    assert!(text.contains("assumed (foreign c \"raylib#IsKeyDown\": runs only on a backend)"), "{text}");
    assert!(text.contains("blocked[B-FOREIGN]"), "{text}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn build_prints_a_game_app_with_its_libraries_runtimes() {
    let dir = project("build");
    let out_dir = dir.join("out");
    let out = Command::new(env!("CARGO_BIN_EXE_spec-lang"))
        .arg("build")
        .arg(&dir)
        .arg("--out")
        .arg(&out_dir)
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{err}");
    for file in ["Cargo.toml", "src/manifest.rs", "src/libs/game/mod.rs", "src/libs/raylib/mod.rs"] {
        assert!(out_dir.join("G").join(file).is_file(), "{file}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn view_and_parse_take_the_new_forms() {
    let dir = project("view");
    let out = Command::new(env!("CARGO_BIN_EXE_spec-lang")).arg("view").arg(&dir).arg("key_down").output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert!(text.contains("foreign:  c \"raylib#IsKeyDown\""), "{text}");
    assert!(text.contains("named by: g.read_pad"), "{text}");
    let (out, text) = run(&["parse"], &dir);
    assert_eq!(out.status.code(), Some(0), "{text}");
    std::fs::remove_dir_all(&dir).unwrap();
}
