//! Finding `.slg` files and naming their modules by path.

use crate::error::IrError;
use crate::library::LibPath;
use crate::lower::lower_program;
use crate::program::Program;
use spec_lang_syntax::SourceMap;
use std::path::{Path, PathBuf};

/// Loading stopped: the errors, and the sources their spans point into.
#[derive(Debug)]
pub struct LoadFailure {
    pub sources: SourceMap,
    pub errors: Vec<IrError>,
}

impl LoadFailure {
    fn plain(e: IrError) -> Self {
        LoadFailure { sources: SourceMap::new(), errors: vec![e] }
    }

    /// One line per error.
    pub fn render(&self) -> String {
        self.errors.iter().map(|e| e.human(&self.sources)).collect::<Vec<_>>().join("\n")
    }
}

/// Load every `.slg` file under `dir` (or the one file `dir` names). A file
/// `a/b.slg` under the directory is module `a.b`; a single file is the module
/// named by its stem. Libraries come from [`LibPath::for_project`].
pub fn load_project(dir: &Path) -> Result<Program, LoadFailure> {
    load_project_with(dir, &LibPath::for_project(dir))
}

/// [`load_project`] with an explicit library search path.
pub fn load_project_with(dir: &Path, libs: &LibPath) -> Result<Program, LoadFailure> {
    if dir.is_file() {
        let name = dir.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        return load_sources_with(vec![(name, dir.display().to_string(), read(dir)?)], libs);
    }
    if !dir.is_dir() {
        return Err(LoadFailure::plain(IrError::load(format!("{}: no such file or directory", dir.display()))));
    }
    let mut files = Vec::new();
    walk(dir, &mut files).map_err(|e| LoadFailure::plain(IrError::load(format!("{}: {e}", dir.display()))))?;
    let mut sources = Vec::new();
    for f in files {
        let rel = f.strip_prefix(dir).unwrap_or(&f).with_extension("");
        let name = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join(".");
        sources.push((name, f.display().to_string(), read(&f)?));
    }
    if sources.is_empty() {
        return Err(LoadFailure::plain(IrError::load(format!("{}: no .slg files", dir.display()))));
    }
    load_sources_with(sources, libs)
}

/// Load one file as one module.
pub fn load_file(path: &Path) -> Result<Program, LoadFailure> {
    load_project_with(path, &LibPath::for_project(path))
}

/// Load modules from `(module name, path for messages, text)` triples.
/// Libraries come from [`LibPath::from_env`].
pub fn load_sources(sources: Vec<(String, String, String)>) -> Result<Program, LoadFailure> {
    load_sources_with(sources, &LibPath::from_env())
}

/// [`load_sources`] with an explicit library search path.
pub fn load_sources_with(sources: Vec<(String, String, String)>, libs: &LibPath) -> Result<Program, LoadFailure> {
    lower_program(sources, libs)
}

pub(crate) fn read(path: &Path) -> Result<String, LoadFailure> {
    std::fs::read_to_string(path)
        .map(|t| t.replace("\r\n", "\n"))
        .map_err(|e| LoadFailure::plain(IrError::load(format!("{}: {e}", path.display()))))
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?.map(|e| e.map(|e| e.path())).collect::<Result<_, _>>()?;
    entries.sort();
    for e in entries {
        if e.is_dir() {
            walk(&e, out)?;
        } else if e.extension().is_some_and(|x| x == "slg") {
            out.push(e);
        }
    }
    Ok(())
}
