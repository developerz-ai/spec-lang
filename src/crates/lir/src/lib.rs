//! The LIR: one typed, language-neutral lowering of a whole program
//! (`docs/decisions/009-one-compiler-any-language.md`).
//!
//! * [`lower`] turns a resolved [`spec_lang_ir::Program`] into a [`Lir`]:
//!   every expression typed, every name resolved, every library operation a
//!   named [`Prim`], rules as explicit world transitions, patterns as tests
//!   and bindings, tests and laws as data.
//! * Each `app` gets a [`Manifest`]: plain data a library's runtime reads.
//! * [`eval`] runs every check of a program on the LIR, so the lowering can
//!   be judged against the engine check by check.
//! * [`print`] renders the LIR as readable text or JSON.
//! * [`Printer`] is what a language's printer implements; `spec-lang build` picks one by target.
//!
//! `README.md` is the contract a printer for a new language implements.

pub mod eval;
mod lower;
pub mod manifest;
mod manifest_of;
pub mod node;
pub mod prim;
pub mod print;
pub mod program;
pub mod target;
pub mod ty;
pub mod walk;

pub use lower::lower;
pub use manifest::Manifest;
pub use prim::Prim;
pub use program::*;
pub use target::{OutFile, Printer};
pub use ty::{ClaimId, DTy, LTy};
