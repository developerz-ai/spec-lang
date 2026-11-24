//! The input script of `run`, `draw` and `shot`: lines of `<ticks> <keys>`, one key letter per `Bool` field of the
//! tick rule's parameter (its first letter in upper case), read whole and checked before the first tick.

use crate::rt::data::Data;

/// The most ticks one line may hold, and a script in all.
pub const MAX_HOLD: u64 = 100_000;
pub const MAX_TOTAL: u64 = 1_000_000;

/// Hold these controls for this many ticks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hold {
    pub ticks: u64,
    pub pad: Data,
}

/// ASCII digits only, at least one; a value beyond the word is the largest.
pub fn digits(text: &[u8]) -> Option<u64> {
    if text.is_empty() || !text.iter().all(u8::is_ascii_digit) {
        return None;
    }
    Some(text.iter().fold(0u64, |n, d| n.saturating_mul(10).saturating_add(u64::from(d - b'0'))))
}

/// The controls `held` names, as the tick rule's parameter.
pub fn pad(fields: &[&'static str], held: &[bool]) -> Data {
    Data::Record(fields.iter().zip(held).map(|(n, h)| (*n, Data::Bool(*h))).collect())
}

fn letter(field: &str) -> Option<char> {
    field.chars().next().and_then(|c| c.to_uppercase().next())
}

/// Left to right: the first character that is a problem, unknown or seen before, decides.
fn keys(fields: &[&'static str], text: &[u8]) -> Result<Vec<bool>, String> {
    let mut held = vec![false; fields.len()];
    if text == b"-" {
        return Ok(held);
    }
    for c in String::from_utf8_lossy(text).chars() {
        match fields.iter().position(|f| letter(f) == Some(c)) {
            None => return Err(format!("unknown key \"{c}\"")),
            Some(i) if held[i] => return Err(format!("key \"{c}\" twice")),
            Some(i) => held[i] = true,
        }
    }
    Ok(held)
}

fn line(fields: &[&'static str], text: &[u8]) -> Result<Option<Hold>, String> {
    let code = text.split(|b| *b == b'#').next().unwrap_or_default();
    let parts: Vec<&[u8]> = code.split(|b| *b == b' ' || *b == b'\t').filter(|f| !f.is_empty()).collect();
    match parts[..] {
        [] => Ok(None),
        [ticks, held] => {
            let ticks = digits(ticks)
                .filter(|n| (1..=MAX_HOLD).contains(n))
                .ok_or_else(|| format!("ticks must be 1 to {MAX_HOLD}"))?;
            Ok(Some(Hold { ticks, pad: pad(fields, &keys(fields, held)?) }))
        }
        _ => Err("expected <ticks> <keys>".into()),
    }
}

/// The whole script, or the message after `error: `.
pub fn read(fields: &[&'static str], input: &[u8]) -> Result<Vec<Hold>, String> {
    let mut holds = Vec::new();
    let mut total = 0u64;
    for (n, text) in input.split(|b| *b == b'\n').enumerate() {
        if let Some(hold) = line(fields, text).map_err(|why| format!("input line {}: {why}", n + 1))? {
            total += hold.ticks;
            holds.push(hold);
        }
    }
    if total > MAX_TOTAL {
        return Err(format!("input: more than {MAX_TOTAL} ticks"));
    }
    Ok(holds)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIELDS: &[&str] = &["left", "right", "jump"];

    #[test]
    fn holds_in_order_comments_and_blank_lines_skipped() {
        let holds =
            read(FIELDS, b"# walk\n\n 3\tR # on\n002 JL\n1 -").map(|hs| hs.iter().map(|h| h.ticks).collect::<Vec<_>>());
        assert_eq!(holds, Ok(vec![3, 2, 1]));
    }

    #[test]
    fn the_first_problem_decides() {
        let why = |text: &str| read(FIELDS, text.as_bytes()).map(|_| ());
        assert_eq!(why("1 R\n1"), Err("input line 2: expected <ticks> <keys>".into()));
        assert_eq!(why("0 R"), Err("input line 1: ticks must be 1 to 100000".into()));
        assert_eq!(why("1 RRZ"), Err("input line 1: key \"R\" twice".into()));
        assert_eq!(why("1 RZR"), Err("input line 1: unknown key \"Z\"".into()));
        assert_eq!(why(&"100000 -\n".repeat(11)), Err("input: more than 1000000 ticks".into()));
    }
}
