// Projected by hand from vocabulary cli: exit codes (docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

/// How the process ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    Success,
    /// A rule refused (`fail`), or the lock was contended.
    Refused,
    Usage,
    Store,
}

impl Exit {
    pub const fn code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::Refused => 1,
            Self::Usage => 2,
            Self::Store => 3,
        }
    }
}
