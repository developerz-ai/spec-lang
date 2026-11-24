// Projected by hand from note @store(file(".todo.json")) (src/apps/rust.slg) and the TODO_STORE variable (docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// `@store(file(".todo.json"))`, relative to the working directory.
pub const DEFAULT: &str = ".todo.json";
/// The environment variable that replaces it.
pub const VARIABLE: &str = "TODO_STORE";

/// The path as configured: the variable's value if it has one, else the default. Never resolved.
/// The caller reads the environment; nothing here does.
pub fn configured(variable: Option<OsString>) -> PathBuf {
    match variable {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => PathBuf::from(DEFAULT),
    }
}

/// `<path><suffix>`: the lock and the temporary file live beside the store.
pub(super) fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}
