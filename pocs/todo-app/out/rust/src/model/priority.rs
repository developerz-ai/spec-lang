// Projected by hand from type Priority (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

/// `type Priority = Low | Normal | High`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
}

impl Priority {
    /// Every variant, in declaration order.
    pub const ALL: [Self; 3] = [Self::Low, Self::Normal, Self::High];

    /// The variant's name as the spec writes it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Normal => "Normal",
            Self::High => "High",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Low" => Some(Self::Low),
            "Normal" => Some(Self::Normal),
            "High" => Some(Self::High),
            _ => None,
        }
    }
}
