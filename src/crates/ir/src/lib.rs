//! The resolved program: what the interpreter runs and what every emitter prints.
//!
//! [`load_project`] reads a directory of `.slg` files (the path is the module:
//! `todo/model.slg` is `todo.model`), parses each with `spec-lang-syntax`, and
//! lowers the syntax trees into one [`Program`]:
//!
//! * every name is resolved: a bare name in an expression is a local, a
//!   declaration of this module, a name bound by `use`, or a core name, and the
//!   IR says which ([`expr::ExprKind`]); imports are flattened away;
//! * declarations are split by kind into typed tables ([`TypeDecl`], [`StateDecl`],
//!   [`FnDecl`], [`RuleDecl`], [`TestDecl`], [`LawDecl`], [`AppDecl`], ...),
//!   addressed by ids ([`ids`]);
//! * notes are interpreted: `@guard` clauses become an ordered list of
//!   [`Guard`]s, `@cases` a list of [`Case`]s, `@given` a list of [`Given`]s,
//!   `@target` and the notes of a library fields of [`AppDecl`];
//! * `use <name>;` that names no module loads the library `libs/<name>/`
//!   ([`library`]): its notes, triggers, handlers and app shape are data the
//!   lowering checks against; the toolchain knows no library by name;
//! * parameter, field and state types are [`Ty`]s, with claims (`T where p`)
//!   kept as expressions over `self`.
//!
//! What it does not do: infer or check the types of expressions. The IR
//! knows declared types; an expression's type is found by running it.
//! Anything outside the supported subset is an [`IrError`] with code
//! `unsupported`, never a silent drop.

pub mod error;
pub mod expr;
pub mod ids;
pub mod library;
mod load;
mod lower;
pub mod program;
pub mod ty;
pub mod visit;

pub use error::IrError;
pub use expr::*;
pub use ids::*;
pub use library::*;
pub use load::{LoadFailure, load_file, load_project, load_project_with, load_sources, load_sources_with};
pub use program::*;
pub use ty::{Claim, Ty};
