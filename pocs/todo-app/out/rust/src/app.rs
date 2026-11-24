// Projected by hand from app TodoRust (src/apps/rust.slg). Stand-in for spec-lang build output; change the spec, not this file.

use std::fmt;

use crate::board::{Command, Halt, Out, Tally, apply};
use crate::store::{Store, StoreError};

/// Why a command printed nothing.
#[derive(Debug)]
pub enum Failure {
    Halted(Halt),
    Store(StoreError),
}

impl From<Halt> for Failure {
    fn from(halt: Halt) -> Self {
        Self::Halted(halt)
    }
}

impl From<StoreError> for Failure {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Halted(halt) => halt.fmt(f),
            Self::Store(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Failure {}

/// One command against the store: load, the rule, commit. The lines come back
/// only when the new world is on disk, so the caller prints after commit.
///
/// The rule's `@uses` picks the path. A rule that writes takes the exclusive
/// lock and may replace the store. A rule that only reads takes the shared
/// lock and no code that writes the store is reachable from it. `stats` reads
/// nothing of a task but its status, so it folds over the stored tasks and
/// builds none of them.
pub fn execute(store: &Store, command: Command) -> Result<Out, Failure> {
    let mut out = Out::default();
    if command.writes() {
        store.update(|world| apply(world, &mut out, command))??;
    } else if command == Command::Stats {
        store
            .fold(Tally::default(), |tally, task| tally.count(task.status))?
            .report(&mut out);
    } else {
        apply(&mut store.read()?, &mut out, command)?;
    }
    Ok(out)
}
