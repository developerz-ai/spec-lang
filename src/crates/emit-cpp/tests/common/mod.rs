//! Loading the todo POC through the ir crate, with paths relative to the POC
//! (`src/todo/board.slg`), so nothing emitted names a local directory.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use spec_lang_ir::{AppId, Program, load_sources};

/// `pocs/todo-app`, from this crate.
pub fn poc() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../pocs/todo-app")
}

/// The todo program and one of its apps.
pub fn todo(app: &str) -> (Program, AppId) {
    let root = poc();
    let mut files = Vec::new();
    walk(&root.join("src"), &mut files);
    files.sort();
    let sources = files
        .iter()
        .map(|f| {
            let rel = f.strip_prefix(&root).unwrap();
            let module = rel.strip_prefix("src").unwrap().with_extension("");
            let module =
                module.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join(".");
            (module, rel.display().to_string(), std::fs::read_to_string(f).unwrap())
        })
        .collect();
    let prog = load_sources(sources).unwrap_or_else(|f| panic!("{}", f.render()));
    let id = prog.apps.iter().position(|a| a.info.name == app).expect("the app");
    (prog, AppId(id as u32))
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "slg") {
            out.push(p);
        }
    }
}
