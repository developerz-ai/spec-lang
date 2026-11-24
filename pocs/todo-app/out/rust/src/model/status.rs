// Projected by hand from type Status (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

/// `type Status = Open | Done`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Open,
    Done,
}

impl Status {
    /// The variant's name as the spec writes it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::Done => "Done",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Open" => Some(Self::Open),
            "Done" => Some(Self::Done),
            _ => None,
        }
    }
}
