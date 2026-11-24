//! The Rust printer over the LIR (`docs/decisions/009-one-compiler-any-language.md`).
//!
//! [`RustPrinter::tests`] prints a whole program as a Cargo project whose `bin/test` runs every check;
//! [`RustPrinter::app`] prints one app with the runtimes of the libraries it uses. Values are typed Rust: records
//! as structs, sums as enums, `Int` as a checked `i128`, `Text` as `String`, `Map`/`Set` as `BTreeMap`/`BTreeSet`;
//! the program's functions take the machine (`m: &mut Machine`, the world and the handlers' state) when they
//! need it. The runtime in `runtime/rt/` is copied verbatim and names no program; nothing here names a program
//! or a library.

mod app;
mod checks;
mod code;
mod cx;
mod decls;
mod dty;
mod expr;
mod machine;
mod names;
mod pat;
mod prim;
mod project;
mod provenance;
mod stmt;

use spec_lang_diag::Diag;
use spec_lang_lir::{Lir, OutFile, Printer};

/// The Rust printer: `@target(rust)`.
#[derive(Debug)]
pub struct RustPrinter;

impl Printer for RustPrinter {
    fn targets(&self) -> &'static [&'static str] {
        &["rust"]
    }

    fn tests(&self, lir: &Lir) -> Result<Vec<OutFile>, Vec<Diag>> {
        project::tests(lir)
    }

    fn app(&self, lir: &Lir, app: &str) -> Result<Vec<OutFile>, Vec<Diag>> {
        project::app(lir, app)
    }
}
