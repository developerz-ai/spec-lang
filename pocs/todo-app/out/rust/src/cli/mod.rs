// Projected by hand from vocabulary cli (docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

mod commands;
mod exit;
mod help;
mod params;
mod parse;
mod run;
mod usage;

pub use commands::COMMANDS;
pub use exit::Exit;
pub use help::HELP;
pub use params::{CommandSpec, Flag, FlagKind, Parsed, Positional, Value, ValueType};
pub use parse::{Invocation, parse};
pub use run::run;
pub use usage::UsageError;
