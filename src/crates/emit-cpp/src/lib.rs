//! The C++ backend of `spec-lang build`.
//!
//! [`emit`] turns one app of a resolved program into a C++23/CMake project:
//! the typed front (shared with the Rust backend) is lowered into a small typed
//! C++ tree ([`ast`]), printed, formatted once with `clang-format`, and
//! assembled with the fixed runtime shims ([`runtime`]) and `provenance.json`.

pub mod ast;
pub mod format;
pub mod lower;
pub mod runtime;

use spec_lang_diag::Diag;
use spec_lang_emit_rust::front;
pub use spec_lang_emit_rust::provenance::OutFile;
pub use spec_lang_emit_rust::{Options, STRATEGIES};
use spec_lang_ir::{AppDecl, AppId, Program};

/// The project for `app`: every file, as a relative path and bytes. Same input, same bytes.
pub fn emit(program: &Program, app: &AppDecl) -> Result<Vec<OutFile>, Vec<Diag>> {
    emit_with(program, app, &Options::default())
}

/// As [`emit`], with strategies switched off: each falls back to the plain translation.
pub fn emit_with(program: &Program, app: &AppDecl, options: &Options) -> Result<Vec<OutFile>, Vec<Diag>> {
    if let Some(unknown) = options.off.iter().find(|s| !STRATEGIES.contains(&s.as_str())) {
        return Err(vec![Diag::load(format!("unknown strategy {unknown}; known: {}", STRATEGIES.join(", ")))]);
    }
    let Some(index) = program.apps.iter().position(|a| a.info.name == app.info.name) else {
        return Err(vec![Diag::new(
            spec_lang_diag::Severity::Error,
            "E-NAME-UNKNOWN",
            format!("no app {}", app.info.name),
        )]);
    };
    let f = front::build(program, AppId(index as u32))?;
    let ns = app.bin.clone().unwrap_or_else(|| lower::snake(&app.info.name));
    let g = lower::Gen::new(&f, ns, options.clone());
    lower::project::files(&g)
}
