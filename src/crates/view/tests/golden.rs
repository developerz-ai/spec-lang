//! The committed views of the todo program are exactly what the crate prints.
//!
//! `pocs/todo-app/docs/views/` is generated output under version control. This
//! test regenerates it in memory and compares bytes, so a stale view, a
//! hand-edited view and a printer that is not deterministic all fail here.
//! To accept a change: `VIEW_BLESS=1 cargo test -p view --test golden`.

// A helper that cannot load its fixture has nothing to return: failing the test is the handling.
#![allow(clippy::panic)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const REGENERATE: &str = "cd src && VIEW_BLESS=1 cargo test -p view --test golden";

fn todo_app() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../pocs/todo-app")
}

fn load() -> spec_lang_ir::Program {
    match spec_lang_ir::load_project(&todo_app().join("src")) {
        Ok(p) => p,
        Err(e) => panic!("the todo program does not load:\n{}", e.render()),
    }
}

fn generate() -> Vec<view::File> {
    let program = load();
    view::bundle(&program, "todo-app", REGENERATE, view::Inputs::default())
}

fn files_under(dir: &Path, base: &Path, out: &mut BTreeSet<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files_under(&path, base, out);
        } else if let Ok(rel) = path.strip_prefix(base) {
            out.insert(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

#[test]
fn the_same_program_gives_the_same_bytes() {
    assert_eq!(generate(), generate());
}

#[test]
fn committed_views_are_current() {
    let dir = todo_app().join("docs/views");
    let files = generate();
    if std::env::var_os("VIEW_BLESS").is_some() {
        let _ = std::fs::remove_dir_all(&dir);
        view::write_files(&dir, &files).unwrap();
    }
    let mut stale = Vec::new();
    for f in &files {
        match std::fs::read_to_string(dir.join(&f.path)) {
            Ok(on_disk) if on_disk == f.contents => {}
            Ok(_) => stale.push(format!("differs: {}", f.path)),
            Err(_) => stale.push(format!("missing: {}", f.path)),
        }
    }
    let expected: BTreeSet<String> = files.iter().map(|f| f.path.clone()).collect();
    let mut on_disk = BTreeSet::new();
    files_under(&dir, &dir, &mut on_disk);
    for extra in on_disk.difference(&expected) {
        stale.push(format!("not generated: {extra}"));
    }
    assert!(stale.is_empty(), "docs/views is stale; run with VIEW_BLESS=1 to regenerate:\n{}", stale.join("\n"));
}

#[test]
fn no_view_names_a_local_path() {
    let root = todo_app().canonicalize().unwrap();
    let root = root.to_string_lossy();
    for f in generate() {
        assert!(!f.contents.contains(root.as_ref()), "{} holds an absolute path", f.path);
        assert!(!f.contents.contains(env!("CARGO_MANIFEST_DIR")), "{} holds an absolute path", f.path);
    }
}

#[test]
fn the_todo_board_reads_as_designed() {
    let program = load();
    let state = view::diagram(&program, view::DiagramKind::State, None, view::Format::Mermaid, view::Inputs::default())
        .unwrap();
    assert!(state.contains("Open --> Done : done [no in tasks] / out.line"), "{state}");
    assert!(state.contains("Done --> Open : reopen [no in tasks] / out.line"), "{state}");
    assert!(state.contains("[*] --> Open : add / out.line"), "{state}");

    let seq = view::diagram(
        &program,
        view::DiagramKind::Sequence,
        Some("done_and_reopen_move_once"),
        view::Format::Mermaid,
        view::Inputs::default(),
    )
    .unwrap();
    assert!(seq.contains("m_todo_board--xcaller: fail AlreadyDone"), "{seq}");

    let pseudo = view::pseudo(&program, Some("todo.board")).unwrap();
    assert!(pseudo.contains("  require tasks[no].status = Open else fail AlreadyDone\n"), "{pseudo}");
}

#[test]
fn a_subject_that_names_nothing_is_an_error() {
    let program = load();
    let ask = |kind, subject, format| view::diagram(&program, kind, subject, format, view::Inputs::default());
    assert!(matches!(
        ask(view::DiagramKind::Impact, Some("nope"), view::Format::Json),
        Err(view::ViewError::UnknownSubject { .. })
    ));
    assert!(matches!(
        ask(view::DiagramKind::Sequence, None, view::Format::Mermaid),
        Err(view::ViewError::NeedsSubject { .. })
    ));
    assert!(matches!(
        ask(view::DiagramKind::Calls, None, view::Format::PlantUml),
        Err(view::ViewError::NoPrinter { .. })
    ));
    assert!(ask(view::DiagramKind::Impact, Some("board.done"), view::Format::Mermaid).is_ok());
}

/// The emitters' `provenance.json` files, where a generated project exists, are read as they are written.
#[test]
fn generated_projects_have_provenance_the_views_can_read() {
    let program = load();
    let facts = view::facts::Facts::of(&program);
    let Ok(backends) = std::fs::read_dir(todo_app().join("gen")) else { return };
    for dir in backends.flatten() {
        let Ok(text) = std::fs::read_to_string(dir.path().join("provenance.json")) else { continue };
        let name = dir.file_name().to_string_lossy().into_owned();
        let prov = view::provenance::Provenance::parse(&text, &name, "").unwrap_or_else(|e| panic!("{name}: {e}"));
        let done = facts.find("todo.board.done").unwrap();
        let impact = view::impact::impact(&facts, done, std::slice::from_ref(&prov));
        assert!(
            impact.files.iter().any(|f| f.decl == "todo.board.done"),
            "{name}: no emitted span names todo.board.done"
        );
    }
}
