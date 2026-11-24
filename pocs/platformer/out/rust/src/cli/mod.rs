// Projected by hand from app SkyhopRust and the game vocabulary's derived command line (src/apps/rust.slg, docs/game-vocabulary.md). Stand-in for spec-lang build output; change the spec, not this file.
//! Arguments, help and exit codes, exactly as contract section 2 fixes them.

mod exit;
mod help;
mod parse;

pub use exit::Exit;
pub use help::HELP;
pub use parse::{Command, Invocation, Options, Parsed, UsageError, parse};
