//! The printer on every program of the corpus and the two POCs: it prints, the same bytes twice, and nothing it prints
//! switches a lint off. Whether the output means what the engine says is `src/bin/corpus`'s to judge.

#![allow(clippy::panic)]

use spec_lang_diag::Diag;
use spec_lang_emit_rust::RustPrinter;
use spec_lang_lir::{Lir, OutFile, Printer};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn lowered(path: &Path) -> Result<Lir, String> {
    let prog = spec_lang_ir::load_project(path).map_err(|f| f.render())?;
    let errors = spec_lang_check::check(&prog);
    if !errors.is_empty() {
        return Err(format!("{errors:?}"));
    }
    spec_lang_lir::lower(&prog).map_err(|ds| format!("{ds:?}"))
}

fn programs() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> =
        std::fs::read_dir(root().join("src/corpus")).into_iter().flatten().flatten().map(|e| e.path()).collect();
    out.retain(|p| p.is_dir());
    out.sort();
    out.extend(["todo-app", "platformer"].map(|p| root().join("pocs").join(p).join("src")));
    out
}

fn text<'f>(files: &'f [OutFile], path: &str) -> Option<&'f str> {
    files.iter().find(|f| f.path == path).and_then(|f| std::str::from_utf8(&f.bytes).ok())
}

#[test]
fn every_program_prints_the_same_bytes_twice_and_allows_no_lint() {
    let programs = programs();
    assert!(programs.len() >= 40, "{} programs", programs.len());
    for p in programs {
        let lir = lowered(&p);
        let Ok(lir) = lir else { panic_at(&p, lir.err()) };
        let first = RustPrinter.tests(&lir);
        let Ok(files) = first else { panic_at(&p, first.err().map(|d| format!("{d:?}"))) };
        assert_eq!(Ok(&files), RustPrinter.tests(&lir).as_ref().map_err(|_| ()), "{}", p.display());
        for f in &files {
            let body = String::from_utf8_lossy(&f.bytes);
            assert!(!body.contains("#![allow"), "{}: {} switches a lint off", p.display(), f.path);
        }
        assert!(files.iter().any(|f| f.path == "bin/test" && f.executable));
    }
}

fn panic_at(p: &Path, why: Option<String>) -> ! {
    panic!("{}: {}", p.display(), why.unwrap_or_default())
}

#[test]
fn an_app_is_its_program_its_manifest_and_its_libraries_runtimes() {
    for (poc, app, exe, lib) in [
        ("todo-app", "TodoRust", "todo", "src/libs/cli/mod.rs"),
        ("platformer", "SkyhopRust", "skyhop", "build/raylib.rs"),
    ] {
        let p = root().join("pocs").join(poc).join("src");
        let Ok(lir) = lowered(&p) else { panic_at(&p, None) };
        let printed = RustPrinter.app(&lir, app);
        let Ok(files) = printed else { panic_at(&p, printed.err().map(|d| format!("{d:?}"))) };
        assert!(text(&files, "Cargo.toml").is_some_and(|t| t.contains(&format!("name = \"{exe}\""))), "{poc}");
        assert!(text(&files, "src/manifest.rs").is_some(), "{poc}");
        assert!(text(&files, lib).is_some(), "{poc}: {lib}");
    }
}

#[test]
fn provenance_names_real_lines() {
    let p = root().join("pocs/todo-app/src");
    let Ok(lir) = lowered(&p) else { panic_at(&p, None) };
    let Ok(files) = RustPrinter.tests(&lir) else { panic_at(&p, None) };
    let Some(prov) = text(&files, "provenance.json") else { panic_at(&p, Some("no provenance.json".into())) };
    for d in lir
        .fns
        .iter()
        .map(|f| &f.info)
        .chain(lir.rules.iter().map(|r| &r.info))
        .chain(lir.tests.iter().map(|t| &t.info))
    {
        let line = prov.lines().find(|l| l.contains(&format!("\"decl\":\"{}\"", d.qualified)));
        let Some(line) = line else { panic_at(&p, Some(format!("{} has no span", d.qualified))) };
        let field = |k: &str| {
            line.split(&format!("\"{k}\":")).nth(1).map(|s| s.split([',', '}']).next().unwrap_or("").trim_matches('"'))
        };
        let lines = field("file").and_then(|f| text(&files, f)).map(|t| t.lines().collect::<Vec<_>>());
        let (start, end) =
            (field("start").and_then(|n| n.parse::<usize>().ok()), field("end").and_then(|n| n.parse::<usize>().ok()));
        let (Some(lines), Some(start), Some(end)) = (lines, start, end) else { panic_at(&p, Some(line.to_string())) };
        let item = lines.get(start - 1..end).unwrap_or_default().join("\n");
        assert!(item.starts_with("///") && item.contains("pub fn "), "{}: {item}", d.qualified);
    }
}

#[test]
fn a_library_without_a_rust_runtime_is_unsupported() {
    let dir = std::env::temp_dir().join(format!("spec-lang-emit-rust-{}", std::process::id()));
    let lib = dir.join("libs/cli");
    let src = dir.join("src");
    let made = std::fs::create_dir_all(&lib).and_then(|()| std::fs::create_dir_all(&src)).and_then(|()| {
        for f in ["vocab.slg", "contract.slg", "pretend.slg"] {
            std::fs::copy(root().join("libs/cli").join(f), lib.join(f))?;
        }
        std::fs::write(
            src.join("hello.slg"),
            "use cli;\n\n@why(\"greets\")\n@on(cli(\"hi\"))\n@uses(out.write)\npub rule hi() = { out.line(\"hi\"); }\n\n@why(\"one binary\")\n@target(rust)\n@bin(\"hello\")\napp Hello(rules: [hi]);\n",
        )
    });
    assert!(made.is_ok(), "{made:?}");
    let printed = lowered(&src).map(|lir| RustPrinter.app(&lir, "Hello"));
    let _ = std::fs::remove_dir_all(&dir);
    let codes = |ds: &[Diag]| ds.iter().map(|d| d.code).collect::<Vec<_>>();
    match printed {
        Ok(Err(ds)) => assert_eq!(codes(&ds), ["E-UNSUPPORTED"], "{ds:?}"),
        other => panic_at(&src, Some(format!("{other:?}"))),
    }
}
