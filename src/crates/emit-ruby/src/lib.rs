//! The Ruby printer of `spec-lang build` (`docs/decisions/009-one-compiler-any-language.md`): a program's
//! LIR as a Ruby project over a small runtime (`prelude/`), and an app's libraries' runtimes from
//! `libs/<name>/real/ruby/`. It knows Ruby and that runtime; it names no program and no library.

mod app;
mod checks;
mod code;
mod decls;
mod expr;
mod gen_;
mod names;
mod pattern;
mod prim;
mod program;
mod project;
mod stmt;
mod types;

use spec_lang_diag::Diag;
use spec_lang_lir::{Lir, OutFile, Printer};

/// The printer for `@target(ruby)`.
#[derive(Debug)]
pub struct Ruby;

/// The one instance `spec-lang build` lists.
pub static PRINTER: Ruby = Ruby;

impl Printer for Ruby {
    fn targets(&self) -> &'static [&'static str] {
        &["ruby"]
    }

    fn tests(&self, lir: &Lir) -> Result<Vec<OutFile>, Vec<Diag>> {
        let g = gen_::Gen::new(lir);
        project::base(&g).map_err(|d| vec![d])
    }

    fn app(&self, lir: &Lir, app: &str) -> Result<Vec<OutFile>, Vec<Diag>> {
        let g = gen_::Gen::new(lir);
        app::project(&g, app).map_err(|d| vec![d])
    }
}

pub(crate) fn id32(i: usize) -> u32 {
    u32::try_from(i).unwrap_or(u32::MAX)
}
