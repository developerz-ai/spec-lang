//! Differential check against the reference: every `slg` fence in `docs/`
//! and `README.md`, and every `.slg` file of the todo POC and of `libs/`, parses — as it
//! does with the Lark grammar (`pocs/todo-app/scripts/slg/check.py`, which
//! reports 0 failures on the same inputs). Fences are found with the
//! reference's own regex, re-implemented in `spec_lang_syntax::fence`.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use spec_lang_syntax::fence::slg_fences;
use spec_lang_syntax::{FileId, SourceFile, SourceMap, parse_file};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn files(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for e in entries {
        if e.is_dir() {
            files(&e, ext, out);
        } else if e.extension().is_some_and(|x| x == ext) {
            out.push(e);
        }
    }
}

#[test]
fn every_fence_and_slg_file_parses() {
    let root = root();
    let mut md = vec![root.join("README.md")];
    files(&root.join("docs"), "md", &mut md);
    let mut slg = Vec::new();
    files(&root.join("pocs/todo-app/src"), "slg", &mut slg);
    files(&root.join("libs"), "slg", &mut slg);
    let mut map = SourceMap::new();
    let mut texts = Vec::new();
    for f in &md {
        let text = std::fs::read_to_string(f).unwrap().replace("\r\n", "\n");
        for fence in slg_fences(&text) {
            texts.push((f.display().to_string(), fence.body, fence.first_line));
        }
    }
    let fences = texts.len();
    for f in &slg {
        texts.push((f.display().to_string(), std::fs::read_to_string(f).unwrap(), 1));
    }
    let mut failures = Vec::new();
    for (path, text, line) in texts.iter() {
        let id: FileId = map.add(SourceFile::new(path.clone(), text.clone(), *line));
        if let Err(d) = parse_file(id, text) {
            failures.push(d.render(&map));
        }
    }
    assert!(fences > 100, "found only {fences} fences: is the docs tree there?");
    assert!(slg.len() >= 9, "found only {} .slg files", slg.len());
    assert!(failures.is_empty(), "{} of {} do not parse:\n{}", failures.len(), texts.len(), failures.join("\n"));
}
