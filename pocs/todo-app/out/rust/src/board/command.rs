// Projected by hand from the events of rule add, list, done, reopen, rename, remove, clear, stats (src/todo/board.slg) and type Change (src/todo/laws.slg). Stand-in for spec-lang build output; change the spec, not this file.

use std::collections::BTreeSet;
use std::fmt;

use super::{Full, Out, Refusal, World, add, clear, done, list, remove, rename, reopen, stats};
use crate::model::{Priority, Tag, TaskNo, Title};

/// One call of a board rule: the rule's name and its parameters, which are the event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Add {
        title: Title,
        priority: Priority,
        tags: BTreeSet<Tag>,
    },
    List {
        all: bool,
        tag: Option<Tag>,
    },
    Done {
        no: TaskNo,
    },
    Reopen {
        no: TaskNo,
    },
    Rename {
        no: TaskNo,
        title: Title,
    },
    Remove {
        no: TaskNo,
    },
    Clear,
    Stats,
}

/// Why a command stopped without changing the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Halt {
    /// A guard refused.
    Refused(Refusal),
    /// The backend limit: see [`Full`].
    Full,
}

impl From<Refusal> for Halt {
    fn from(refusal: Refusal) -> Self {
        Self::Refused(refusal)
    }
}

impl From<Full> for Halt {
    fn from(_: Full) -> Self {
        Self::Full
    }
}

impl fmt::Display for Halt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused(refusal) => refusal.fmt(f),
            Self::Full => Full.fmt(f),
        }
    }
}

impl std::error::Error for Halt {}

impl Command {
    /// Whether the rule's `@uses` names a write to the board's state.
    pub const fn writes(&self) -> bool {
        !matches!(self, Self::List { .. } | Self::Stats)
    }
}

/// Runs the rule the command names. The replies (`add`'s number, `clear`'s count)
/// have no reader on a command line and stop here.
pub fn apply(world: &mut World, out: &mut Out, command: Command) -> Result<(), Halt> {
    match command {
        Command::Add {
            title,
            priority,
            tags,
        } => {
            add(world, out, title, priority, tags)?;
        }
        Command::List { all, tag } => list(world, out, all, tag.as_ref()),
        Command::Done { no } => done(world, out, no)?,
        Command::Reopen { no } => reopen(world, out, no)?,
        Command::Rename { no, title } => rename(world, out, no, title)?,
        Command::Remove { no } => remove(world, out, no)?,
        Command::Clear => {
            clear(world, out);
        }
        Command::Stats => stats(world, out),
    }
    Ok(())
}
