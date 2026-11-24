// Projected by hand from note @store: "an unreadable store exits 3" and the table "Accepted on read" (docs/cli-vocabulary.md, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

use std::fmt;
use std::io;

/// Why the store's bytes are not a world. The user is told only "unreadable"; the reason is for tests and debugging.
#[derive(Debug)]
pub enum Unreadable {
    /// The file exists and could not be read.
    Io(io::Error),
    NotUtf8,
    /// Not JSON, or JSON of the wrong shape, at this byte.
    Syntax(usize),
    /// A string escape that names no character: a bad `\u`, a lone surrogate.
    Escape(usize),
    UnknownKey,
    DuplicateKey,
    MissingKey,
    /// `version` is not the integer 1.
    Version,
    /// Not an integer from 1 to 2^53: a fraction, an exponent, zero, too large.
    Integer,
    Title,
    Status,
    Priority,
    Tag,
    TagTwice,
    /// A task's number is not below `next_no`.
    NumberNotBelowNext,
    NumberTwice,
}

impl fmt::Display for Unreadable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(why) => write!(f, "cannot be read: {why}"),
            Self::NotUtf8 => f.write_str("not UTF-8"),
            Self::Syntax(at) => write!(f, "not the expected JSON at byte {at}"),
            Self::Escape(at) => write!(f, "bad string escape at byte {at}"),
            Self::UnknownKey => f.write_str("a key the store does not have"),
            Self::DuplicateKey => f.write_str("a key twice in one object"),
            Self::MissingKey => f.write_str("a key is missing"),
            Self::Version => f.write_str("version is not 1"),
            Self::Integer => f.write_str("not an integer from 1 to 2^53"),
            Self::Title => f.write_str("a title that is not a Title"),
            Self::Status => f.write_str("a status that is not a Status"),
            Self::Priority => f.write_str("a priority that is not a Priority"),
            Self::Tag => f.write_str("a tag that is not a Tag"),
            Self::TagTwice => f.write_str("a tag twice on one task"),
            Self::NumberNotBelowNext => f.write_str("a task number not below next_no"),
            Self::NumberTwice => f.write_str("a task number twice"),
        }
    }
}

impl std::error::Error for Unreadable {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(why) => Some(why),
            _ => None,
        }
    }
}
