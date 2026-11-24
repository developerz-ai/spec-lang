// Projected by hand from type Pad (src/hop/model.slg) and the input script of contract section 2. Stand-in for spec-lang build output; change the spec, not this file.

use std::fmt;

use crate::model::Pad;

/// The most ticks one line may hold.
pub const MAX_HOLD: u32 = 100_000;
/// The most ticks a script may hold in all.
pub const MAX_TOTAL: u64 = 1_000_000;

/// One line of the script: hold these keys for this many ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hold {
    pub ticks: u32,
    pub pad: Pad,
}

/// A whole script, checked.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Script {
    pub holds: Vec<Hold>,
}

impl Script {
    /// Every tick's pad, in order.
    pub fn pads(&self) -> impl Iterator<Item = Pad> + '_ {
        self.holds
            .iter()
            .flat_map(|h| std::iter::repeat_n(h.pad, h.ticks as usize))
    }
}

/// Why a line is refused, in the order the contract checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Why {
    Fields,
    Ticks,
    /// The first character that is not a key letter, as given; U+FFFD for bytes that are not UTF-8.
    UnknownKey(String),
    Twice(char),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptError {
    /// `error: input line N: <why>`, `N` from 1.
    Line(usize, Why),
    /// `error: input: more than 1000000 ticks`
    TooLong,
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Line(n, why) => {
                write!(f, "input line {n}: ")?;
                match why {
                    Why::Fields => f.write_str("expected <ticks> <keys>"),
                    Why::Ticks => write!(f, "ticks must be 1 to {MAX_HOLD}"),
                    Why::UnknownKey(c) => write!(f, "unknown key \"{c}\""),
                    Why::Twice(c) => write!(f, "key \"{c}\" twice"),
                }
            }
            Self::TooLong => write!(f, "input: more than {MAX_TOTAL} ticks"),
        }
    }
}

fn ticks(field: &[u8]) -> Option<u32> {
    if field.is_empty() || !field.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let value = field.iter().fold(0u64, |n, d| {
        n.saturating_mul(10).saturating_add(u64::from(d - b'0'))
    });
    u32::try_from(value)
        .ok()
        .filter(|v| (1..=MAX_HOLD).contains(v))
}

/// The bit of each key letter in `Pad` order.
fn key_bit(c: char) -> Option<u8> {
    match c {
        'L' => Some(1),
        'R' => Some(2),
        'F' => Some(4),
        'B' => Some(8),
        'J' => Some(16),
        _ => None,
    }
}

fn keys(field: &[u8]) -> Result<Pad, Why> {
    if field == b"-" {
        return Ok(Pad::default());
    }
    // Left to right: the first character that is a problem, unknown or seen before, decides.
    let mut held = 0u8;
    for c in String::from_utf8_lossy(field).chars() {
        let bit = key_bit(c).ok_or_else(|| Why::UnknownKey(c.to_string()))?;
        if held & bit != 0 {
            return Err(Why::Twice(c));
        }
        held |= bit;
    }
    Ok(Pad {
        left: held & 1 != 0,
        right: held & 2 != 0,
        forward: held & 4 != 0,
        back: held & 8 != 0,
        jump: held & 16 != 0,
    })
}

fn line(text: &[u8]) -> Result<Option<Hold>, Why> {
    let code = text.split(|b| *b == b'#').next().unwrap_or_default();
    let mut fields = code
        .split(|b| *b == b' ' || *b == b'\t')
        .filter(|f| !f.is_empty());
    let (Some(first), second, third) = (fields.next(), fields.next(), fields.next()) else {
        return Ok(None);
    };
    let (Some(second), None) = (second, third) else {
        return Err(Why::Fields);
    };
    let ticks = ticks(first).ok_or(Why::Ticks)?;
    let pad = keys(second)?;
    Ok(Some(Hold { ticks, pad }))
}

/// Reads standard input whole: every line is checked before the first tick runs.
pub fn read(input: &[u8]) -> Result<Script, ScriptError> {
    let mut holds = Vec::new();
    let mut total = 0u64;
    for (n, text) in input.split(|b| *b == b'\n').enumerate() {
        if let Some(hold) = line(text).map_err(|why| ScriptError::Line(n + 1, why))? {
            total += u64::from(hold.ticks);
            holds.push(hold);
        }
    }
    if total > MAX_TOTAL {
        return Err(ScriptError::TooLong);
    }
    Ok(Script { holds })
}
