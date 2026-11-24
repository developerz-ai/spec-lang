//! The one interface every printer implements (`docs/decisions/009-one-compiler-any-language.md`).
//! `spec-lang build` lowers a program once and hands the [`Lir`] to the printer a target names.

use crate::Lir;
use spec_lang_diag::Diag;

/// One file a printer writes, relative to the output directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutFile {
    pub path: String,
    pub bytes: Vec<u8>,
    pub executable: bool,
}

/// A printer for one language. It knows its language and its runtime, never a program or a library by name:
/// a library's runtime comes from `libs/<name>/real/<language>/`, found through the manifest.
pub trait Printer: Sync {
    /// The `@target(...)` names it serves, first one canonical (`["ts", "bun"]`).
    fn targets(&self) -> &'static [&'static str];

    /// A whole program as a project whose `bin/test` runs every check and prints one JSON line per check:
    /// `{"check": "<decl path>" or "<decl path> case[<i>]", "status": "pass" | "fail" | "blocked"}`,
    /// the names and statuses `spec-lang test --json` and `spec-lang lir --eval --json` give.
    fn tests(&self, lir: &Lir) -> Result<Vec<OutFile>, Vec<Diag>>;

    /// One app (by its declared name) as a project that builds the app's executable, with the program's checks
    /// as its own tests. A library the app uses with no runtime for this language is `E-UNSUPPORTED`.
    fn app(&self, lir: &Lir, app: &str) -> Result<Vec<OutFile>, Vec<Diag>>;
}
