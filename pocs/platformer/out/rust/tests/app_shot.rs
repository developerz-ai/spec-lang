// Projected by hand from the shot command the game vocabulary derives from tick and frame (docs/game-vocabulary.md, src/apps/rust.slg) and load_assets (docs/contract.md section 4). Stand-in for spec-lang build output; change the spec, not this file.
//! The built binary, through raylib, in a real (virtual) X display. Skipped, with the reason printed, when there is none.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn skyhop(args: &[&str], stdin: &str, assets: &Path) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_skyhop"))
        .args(args)
        .env("SKYHOP_ASSETS", assets)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn display() -> bool {
    let named = std::env::var_os("DISPLAY").is_some_and(|d| !d.is_empty());
    if !named {
        eprintln!(
            "skipped: no X server (DISPLAY is not set); bin/check runs the tests under xvfb-run -a"
        );
    }
    named
}

#[test]
fn a_shot_is_stable_and_written_as_png() {
    if !display() {
        return;
    }
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let png = dir.join("shot-150.png");
    let script = "75 R\n20 RJ\n41 R\n20 RJ\n35 R\n";
    let png_arg = png.to_str().unwrap();
    let first = skyhop(
        &["shot", "--at", "150", "--out", png_arg],
        script,
        &assets(),
    );
    let second = skyhop(&["shot", "--at", "150"], script, &assets());
    let stdout = String::from_utf8(first.stdout).unwrap();
    assert_eq!(
        first.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(
        stdout.starts_with("shot ") && stdout.len() == 22 && stdout.ends_with('\n'),
        "{stdout:?}"
    );
    assert_eq!(String::from_utf8(second.stdout).unwrap(), stdout);
    let bytes = std::fs::read(&png).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let start = skyhop(&["shot"], "", &assets());
    assert_ne!(
        String::from_utf8(start.stdout).unwrap(),
        stdout,
        "tick 0 and tick 150 draw alike"
    );
}

#[test]
fn a_missing_asset_is_exit_3() {
    if !display() {
        return;
    }
    let missing = Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-assets");
    let r = skyhop(&["shot"], "", &missing);
    assert_eq!(r.status.code(), Some(3));
    assert_eq!(r.stdout, b"");
    let want = format!(
        "error: asset {}: cannot load\n",
        missing.join("shaders/lit.vs").display()
    );
    assert_eq!(String::from_utf8(r.stderr).unwrap(), want);
}

#[test]
fn no_display_is_exit_3() {
    let r = Command::new(env!("CARGO_BIN_EXE_skyhop"))
        .arg("shot")
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(r.status.code(), Some(3));
    assert_eq!(r.stdout, b"");
    assert_eq!(r.stderr, b"error: window: cannot open\n");
}
