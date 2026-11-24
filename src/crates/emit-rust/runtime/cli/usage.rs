//! Usage errors of the `cli` vocabulary (docs/contract.md section 2).

use std::fmt;

/// Every way a command line can be wrong. The rule never ran, so nothing was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UsageError {
    /// An argument (counted from 1) is not Unicode text, so it is no `Text` at all.
    NotUnicode(usize),
    UnknownCommand(String),
    /// The whole argument as given, dashes included.
    UnknownFlag(String),
    GivenTwice(&'static str),
    NeedsValue(&'static str),
    FlagNotA {
        flag: &'static str,
        type_name: &'static str,
    },
    Unexpected(String),
    PositionalNotA {
        name: &'static str,
        type_name: &'static str,
    },
    Missing(&'static str),
}

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotUnicode(position) => write!(f, "argument {position}: not UTF-8"),
            Self::UnknownCommand(given) => write!(f, "unknown command \"{given}\""),
            Self::UnknownFlag(given) => write!(f, "unknown flag {given}"),
            Self::GivenTwice(flag) => write!(f, "--{flag} given twice"),
            Self::NeedsValue(flag) => write!(f, "--{flag} needs a value"),
            Self::FlagNotA { flag, type_name } => write!(f, "--{flag}: not a {type_name}"),
            Self::Unexpected(given) => write!(f, "unexpected argument \"{given}\""),
            Self::PositionalNotA { name, type_name } => write!(f, "<{name}>: not a {type_name}"),
            Self::Missing(name) => write!(f, "missing <{name}>"),
        }
    }
}

impl std::error::Error for UsageError {}
