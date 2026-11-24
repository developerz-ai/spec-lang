// Projected by hand from type TaskNo (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

use std::fmt;

/// `type TaskNo = Int where self >= 1`.
///
/// `Int` is exact and unbounded; this is one machine word. The licence is the
/// contract's store limit: `next_no` never passes 2^53 and every stored number
/// is below `next_no`, so every number in any world fits with room to spare.
/// The only larger numbers are command-line arguments, and a rule does one
/// thing with such an argument: look it up. Every literal past the word is
/// therefore the same value, [`TaskNo::BEYOND`], which is in no world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskNo(u64);

impl TaskNo {
    /// The initial `next_no`.
    pub const FIRST: Self = Self(1);
    /// 2^53: the largest `next_no` a store holds (docs/contract.md section 3).
    pub const LIMIT: Self = Self(1 << 53);
    /// Stands for every number too large for the word; above `LIMIT`, so never in a world.
    pub const BEYOND: Self = Self(u64::MAX);

    /// `None` for 0, the one word outside the claim.
    pub const fn new(value: u64) -> Option<Self> {
        if value >= 1 { Some(Self(value)) } else { None }
    }

    /// ASCII digits only, one or more, value at least 1; leading zeros allowed.
    pub fn parse(text: &str) -> Option<Self> {
        if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let digits = text.trim_start_matches('0');
        if digits.is_empty() {
            return None;
        }
        Some(digits.parse().map_or(Self::BEYOND, Self))
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    /// `no + 1`, or `None` at `LIMIT` and above, where the store can no longer follow.
    pub const fn successor(self) -> Option<Self> {
        if self.0 < Self::LIMIT.0 {
            Some(Self(self.0 + 1))
        } else {
            None
        }
    }
}

impl fmt::Display for TaskNo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
