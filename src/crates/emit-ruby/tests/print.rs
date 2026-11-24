//! The printer on real programs: what it writes is stable, parses as Ruby without a warning, and an app gets
//! its libraries' runtimes. The corpus judge (`src/bin/corpus`) says the printed checks agree with the engine.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use emit_ruby::PRINTER;
use spec_lang_lir::{Lir, OutFile, Printer};
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn lower(project: &Path) -> Lir {
    let prog = spec_lang_ir::load_project(project).unwrap_or_else(|f| panic!("{}", f.render()));
    let errors = spec_lang_check::check(&prog);
    assert!(errors.is_empty(), "{project:?} does not check");
    spec_lang_lir::lower(&prog).unwrap_or_else(|ds| panic!("{project:?} does not lower: {ds:?}"))
}

fn write(dir: &Path, files: &[OutFile]) {
    for f in files {
        let path = dir.join(&f.path);
        std::fs::create_dir_all(path.parent().expect("a file has a folder")).expect("mkdir");
        std::fs::write(&path, &f.bytes).expect("write");
        #[cfg(unix)]
        if f.executable {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        }
    }
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("emit-ruby-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// Whether `ruby` runs here; without it the tests that run Ruby say so and stop.
fn ruby() -> bool {
    let found = Command::new("ruby").arg("--version").output().is_ok_and(|o| o.status.success());
    if !found {
        eprintln!("ruby is not installed: the printed files were not run");
    }
    found
}

fn path<'a>(files: &'a [OutFile], path: &str) -> &'a OutFile {
    files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no file {path}"))
}

#[test]
fn the_same_program_prints_the_same_bytes() {
    let project = repo().join("pocs/todo-app/src");
    let (one, two) =
        (PRINTER.tests(&lower(&project)).expect("prints"), PRINTER.tests(&lower(&project)).expect("prints"));
    assert_eq!(one, two);
    let mut paths: Vec<&str> = one.iter().map(|f| f.path.as_str()).collect();
    let all = paths.len();
    paths.sort_unstable();
    paths.dedup();
    assert_eq!(paths.len(), all, "a path is written twice");
}

#[test]
fn every_file_names_its_source_and_no_path_of_this_machine() {
    let files = PRINTER.app(&lower(&repo().join("pocs/todo-app/src")), "TodoRuby").expect("prints");
    let board = String::from_utf8(path(&files, "lib/todo/board.rb").bytes.clone()).expect("utf-8");
    assert!(board.contains("from module todo.board (todo/board.slg)"), "{board}");
    assert!(board.contains("# rule add (todo/board.slg:21)"), "{board}");
    let here = repo().canonicalize().expect("the checkout").to_string_lossy().into_owned();
    for f in &files {
        let text = String::from_utf8_lossy(&f.bytes);
        assert!(!text.contains(&here), "{} holds a path of this machine", f.path);
    }
}

#[test]
fn an_app_gets_its_libraries_runtimes_and_an_executable() {
    let files = PRINTER.app(&lower(&repo().join("pocs/todo-app/src")), "TodoRuby").expect("prints");
    assert!(path(&files, "exe/todo").executable);
    assert!(path(&files, "bin/test").executable);
    path(&files, "lib/libs/cli.rb");
    path(&files, "lib/libs/cli/store/codec.rb");
    path(&files, "test/libs/cli/store_test.rb");
    let glue = String::from_utf8(path(&files, "lib/app.rb").bytes.clone()).expect("utf-8");
    assert!(glue.contains("PROGRAM.apps.fetch(\"TodoRuby\")") && glue.contains("LIBRARIES = [Libs::Cli]"), "{glue}");
    assert!(files.iter().all(|f| !f.path.ends_with("runtime.conf") && !f.path.ends_with("README.md")));

    let game = PRINTER.app(&lower(&repo().join("pocs/platformer/src")), "SkyhopRuby").expect("prints");
    assert!(path(&game, "exe/skyhop").executable);
    let glue = String::from_utf8(path(&game, "lib/app.rb").bytes.clone()).expect("utf-8");
    assert!(glue.contains("LIBRARIES = [Libs::Game, Libs::Raylib]"), "{glue}");
    let gemfile = String::from_utf8(path(&game, "Gemfile").bytes.clone()).expect("utf-8");
    assert!(gemfile.contains("gem \"ffi\", \"1.17.4\""), "{gemfile}");
}

#[test]
fn an_app_that_is_not_there_is_refused() {
    let lir = lower(&repo().join("pocs/todo-app/src"));
    let errors = PRINTER.app(&lir, "NoSuchApp").expect_err("no such app");
    assert_eq!(errors[0].code, "E-UNSUPPORTED");
}

#[test]
fn every_printed_file_parses_without_a_warning() {
    if !ruby() {
        return;
    }
    for (name, files) in [
        ("todo", PRINTER.app(&lower(&repo().join("pocs/todo-app/src")), "TodoRuby").expect("prints")),
        ("skyhop", PRINTER.app(&lower(&repo().join("pocs/platformer/src")), "SkyhopRuby").expect("prints")),
        ("loops", PRINTER.tests(&lower(&repo().join("src/corpus/f19-loops-var-return"))).expect("prints")),
    ] {
        let dir = scratch(name);
        write(&dir, &files);
        let ruby_files: Vec<PathBuf> = files
            .iter()
            .filter(|f| f.path.ends_with(".rb") || f.path.starts_with("exe/") || f.path == "bin/test")
            .map(|f| dir.join(&f.path))
            .collect();
        let out = Command::new("ruby").arg("-wc").args(&ruby_files).output().expect("ruby runs");
        let said = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        assert!(out.status.success() && !said.contains("warning"), "{name}: {said}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn the_printed_checks_run_and_report_one_line_each() {
    if !ruby() {
        return;
    }
    let lir = lower(&repo().join("src/corpus/f24-rule-atomic"));
    let dir = scratch("atomic");
    write(&dir, &PRINTER.tests(&lir).expect("prints"));
    let out = Command::new(dir.join("bin/test")).args(["--jobs", "1"]).output().expect("bin/test runs");
    let said = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = said.lines().collect();
    assert_eq!(lines.len(), lir.checks.len(), "{said}{}", String::from_utf8_lossy(&out.stderr));
    for (line, check) in lines.iter().zip(&lir.checks) {
        assert!(line.contains(&format!("{{\"check\": \"{}\"", check.name)), "{line}");
        assert!(line.contains("\"status\": \"pass\""), "{line}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
