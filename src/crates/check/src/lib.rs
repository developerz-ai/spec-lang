//! Stages 2 and 3 of `spec-lang test`, after name resolution in the IR:
//!
//! * [`check`] (errors): every `pub` rule writes `@uses`, and what it lists
//!   covers the effects its body has, called rules and fns included; every
//!   default, constant and state initial value is of its declared type.
//! * [`lint`] (warnings): `pub` rules without `@why`, unused `use` items and
//!   private declarations, tests that expect nothing, laws whose trace names
//!   a rule that cannot change what the law reads, declared effects the body
//!   never has.
//!
//! Name, note, guard-form, arity and state-ownership errors come from the IR
//! loader itself; this crate adds what needs the whole program.

mod defaults;
pub mod effects;
mod lint;
pub mod refs;

use spec_lang_diag::Diag;
use spec_lang_ir::Program;

/// Errors of the check stage, in declaration order.
pub fn check(prog: &Program) -> Vec<Diag> {
    let mut out = effects::check_uses(prog);
    out.extend(defaults::check_defaults(prog));
    out
}

/// Warnings of the lint stage, in declaration order.
pub fn lint(prog: &Program) -> Vec<Diag> {
    lint::lint(prog)
}
