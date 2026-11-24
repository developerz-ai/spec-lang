//! The todo POC through the C++ backend: emitted, deterministic, every file
//! valid C++, every generated line covered by provenance. `write_gen` (ignored,
//! run explicitly) writes the project to `pocs/todo-app/gen/cpp`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use spec_lang_emit_cpp::{Options, OutFile, emit_with};

fn todo_files() -> Vec<OutFile> {
    todo_files_with(&Options::default())
}

fn todo_files_with(options: &Options) -> Vec<OutFile> {
    let (prog, app) = common::todo("TodoCpp");
    emit_with(&prog, prog.app(app), options).unwrap_or_else(|ds| {
        panic!(
            "{}",
            ds.iter()
                .map(|d| format!("{d} at {:?} in {:?}: {:?}", d.span.map(|s| prog.location(s)), d.decl, d.notes))
                .collect::<Vec<_>>()
                .join("\n")
        )
    })
}

fn write(files: &[OutFile], root: &Path) {
    if root.exists() {
        for entry in std::fs::read_dir(root).unwrap() {
            let p = entry.unwrap().path();
            if p.file_name().is_some_and(|n| n == "build") {
                continue;
            }
            if p.is_dir() { std::fs::remove_dir_all(&p).unwrap() } else { std::fs::remove_file(&p).unwrap() }
        }
    }
    for f in files {
        let path = root.join(&f.path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &f.bytes).unwrap();
        if f.executable {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
}

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn same_input_same_bytes() {
    assert_eq!(todo_files(), todo_files());
}

#[test]
fn nothing_names_a_local_directory() {
    let home = std::env::var("HOME").unwrap_or_default();
    for f in todo_files() {
        let text = String::from_utf8_lossy(&f.bytes);
        assert!(home.is_empty() || !text.contains(&home), "{} names {home}", f.path);
    }
}

#[test]
fn every_file_is_valid_cpp() {
    syntax_check(&todo_files(), "syntax");
}

/// Every strategy off: the plain translation still compiles. Its parity is run on `write_gen_plain`.
#[test]
fn the_plain_translation_is_valid_cpp() {
    let plain = todo_files_with(&Options::plain());
    assert_ne!(plain, todo_files(), "switching every strategy off changes the output");
    syntax_check(&plain, "syntax-plain");
}

#[test]
fn an_unknown_strategy_is_an_error() {
    let (prog, app) = common::todo("TodoCpp");
    let options = Options { off: ["no.such".to_string()].into() };
    assert!(emit_with(&prog, prog.app(app), &options).is_err());
}

fn syntax_check(files: &[OutFile], name: &str) {
    let dir = scratch(name);
    write(files, &dir);
    // The compiler that builds the project, as CMake picks it: `CXX`, else the system `c++`.
    // A fixed `clang++` fails where clang is older than the C++ library (clang 18 cannot use GCC 13's <expected>).
    let cxx = std::env::var("CXX").unwrap_or_else(|_| "c++".to_string());
    for f in files.iter().filter(|f| f.path.ends_with(".cpp") || f.path.ends_with(".hpp")) {
        // A header is checked as build systems check it: through a file that includes it, so it is
        // self-contained and `#pragma once` is not in the main file.
        let unit = if f.path.ends_with(".hpp") {
            let probe = format!("{}.probe.cpp", f.path);
            std::fs::write(
                dir.join(&probe),
                format!("#include \"{}\"\n", Path::new(&f.path).file_name().unwrap().to_string_lossy()),
            )
            .unwrap();
            probe
        } else {
            f.path.clone()
        };
        let header_dir = Path::new(&f.path).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        let out = Command::new(&cxx)
            .args([
                "-std=c++23",
                "-fsyntax-only",
                "-Wall",
                "-Wextra",
                "-Wpedantic",
                "-Werror",
                "-Iinclude",
                "-Itests",
                "-Itests/rt",
            ])
            .arg(format!("-I{header_dir}"))
            .arg(&unit)
            .current_dir(&dir)
            .output()
            .unwrap_or_else(|e| panic!("{cxx}: {e}"));
        assert!(out.status.success(), "{}:\n{}", f.path, String::from_utf8_lossy(&out.stderr));
    }
}

#[test]
fn every_generated_line_has_provenance() {
    let files = todo_files();
    let prov = files.iter().find(|f| f.path == "provenance.json").expect("provenance.json");
    let text = String::from_utf8(prov.bytes.clone()).unwrap();
    for f in files.iter().filter(|f| f.path != "provenance.json") {
        let lines = String::from_utf8_lossy(&f.bytes).lines().count();
        let mut covered = vec![false; lines + 1];
        for row in text.lines().filter(|l| l.contains(&format!("\"file\":\"{}\"", f.path))) {
            let num = |key: &str| -> usize {
                let at = row.find(&format!("\"{key}\":")).unwrap() + key.len() + 3;
                row[at..].split(|c: char| !c.is_ascii_digit()).next().unwrap().parse().unwrap()
            };
            covered[num("start")..=num("end")].fill(true);
        }
        let missing: Vec<usize> = (1..=lines).filter(|l| !covered[*l]).collect();
        assert!(missing.is_empty(), "{}: lines {missing:?} have no span", f.path);
    }
}

#[test]
#[ignore = "writes pocs/todo-app/gen/cpp; run with --ignored"]
fn write_gen() {
    write(&todo_files(), &common::poc().join("gen/cpp"));
}

#[test]
#[ignore = "writes the plain translation to the build's temporary directory; run with --ignored"]
fn write_gen_plain() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("gen-cpp-plain");
    std::fs::create_dir_all(&dir).unwrap();
    write(&todo_files_with(&Options::plain()), &dir);
    println!("{}", dir.display());
}
