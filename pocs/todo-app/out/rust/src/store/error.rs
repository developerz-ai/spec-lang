// Projected by hand from note @store: its errors (docs/cli-vocabulary.md, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

use std::fmt;
use std::io;

use super::codec::Unreadable;

/// What can go wrong between the rule and the disk. None of these is a spec error.
#[derive(Debug)]
pub enum StoreError {
    /// The lock was not had within the patience allowed.
    Contended,
    /// The store cannot be read, or what it holds is not a world.
    Unreadable(Unreadable),
    /// The store or its lock cannot be created or written.
    Unwritable(io::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contended => f.write_str("Contended"),
            Self::Unreadable(_) => f.write_str("unreadable"),
            Self::Unwritable(_) => f.write_str("unwritable"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Contended => None,
            Self::Unreadable(why) => Some(why),
            Self::Unwritable(why) => Some(why),
        }
    }
}

impl From<Unreadable> for StoreError {
    fn from(why: Unreadable) -> Self {
        Self::Unreadable(why)
    }
}
