// Projected by hand from errors NoTask, AlreadyDone, NotDone of rule done, reopen, rename, remove (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

use std::fmt;

/// The errors the board's guards declare. A refused rule has changed nothing and sent nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    NoTask,
    AlreadyDone,
    NotDone,
}

impl Refusal {
    /// The error's name as the spec writes it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::NoTask => "NoTask",
            Self::AlreadyDone => "AlreadyDone",
            Self::NotDone => "NotDone",
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl std::error::Error for Refusal {}
