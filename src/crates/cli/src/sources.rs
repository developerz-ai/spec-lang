//! Finding the texts `parse` checks: `.slg` files whole, `.md` files by fence.

use spec_lang_syntax::fence::slg_fences;
use std::path::{Path, PathBuf};

/// One text to parse: where it came from and its first line number.
#[derive(Debug)]
pub struct Source {
    pub path: String,
    pub text: String,
    pub first_line: u32,
}

/// Every `.slg` file and `slg` fence under `paths`, in sorted path order.
pub fn collect(paths: &[String]) -> Result<Vec<Source>, String> {
    let mut files = Vec::new();
    for p in paths {
        let path = Path::new(p);
        if path.is_dir() {
            walk(path, &mut files).map_err(|e| format!("{p}: {e}"))?;
        } else if path.is_file() {
            files.push(path.to_path_buf());
        } else {
            return Err(format!("{p}: no such file or directory"));
        }
    }
    let mut out = Vec::new();
    for f in files {
        let ext = f.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "slg" && ext != "md" {
            continue;
        }
        let text = read(&f)?;
        let name = f.display().to_string();
        if ext == "slg" {
            out.push(Source { path: name, text, first_line: 1 });
        } else {
            for fence in slg_fences(&text) {
                out.push(Source { path: name.clone(), text: fence.body, first_line: fence.first_line });
            }
        }
    }
    Ok(out)
}

/// A text file with `\r\n` read as `\n`, as Python's text mode reads it.
pub fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map(|t| t.replace("\r\n", "\n")).map_err(|e| format!("{}: {e}", path.display()))
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?.map(|e| e.map(|e| e.path())).collect::<Result<_, _>>()?;
    entries.sort();
    for e in entries {
        if e.is_dir() {
            walk(&e, out)?;
        } else {
            out.push(e);
        }
    }
    Ok(())
}
