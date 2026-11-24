// Projected by hand from the exit codes of contract section 2 (docs/contract.md). Stand-in for spec-lang build output; change the spec, not this file.

/// The process exit codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    /// 0
    Success,
    /// 1: standard output could not be written. Not in the contract; see NOTES.md.
    Output,
    /// 2: usage or input
    Usage,
    /// 3: the window or an asset cannot be opened
    Window,
}

impl Exit {
    pub const fn code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::Output => 1,
            Self::Usage => 2,
            Self::Window => 3,
        }
    }
}
