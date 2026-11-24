//! What can go wrong between a rule and the disk. None of these is a spec error.

use std::fmt;
use std::io;

use super::Unreadable;

#[derive(Debug)]
pub enum StoreError {
    /// The lock was not had within the patience allowed.
    Contended,
    /// The store cannot be read, or what it holds is not a world.
    Unreadable(Unreadable),
    /// The store or its lock cannot be created or written.
    Unwritable(io::Error),
    /// The world holds an `Int` above 2^53, which the store format cannot carry.
    Full,
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contended => f.write_str("Contended"),
            Self::Unreadable(_) => f.write_str("unreadable"),
            Self::Unwritable(_) => f.write_str("unwritable"),
            Self::Full => f.write_str("full"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Unreadable(why) => Some(why),
            Self::Unwritable(why) => Some(why),
            Self::Contended | Self::Full => None,
        }
    }
}

impl From<Unreadable> for StoreError {
    fn from(why: Unreadable) -> Self {
        Self::Unreadable(why)
    }
}
