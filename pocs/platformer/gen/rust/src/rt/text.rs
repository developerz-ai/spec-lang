//! `Text` by code point. `String`'s order is byte order, which for UTF-8 is code point order.

use super::{Int, R, int::count, stop};

pub fn size(t: &str) -> Int {
    count(t.chars().count())
}

pub fn bytes(t: &str) -> Vec<Int> {
    t.bytes().map(Int::from).collect()
}

/// Removes the 25 `White_Space` code points from both ends (`char::is_whitespace` is exactly them).
pub fn trim(t: &str) -> String {
    t.trim_matches(char::is_whitespace).to_string()
}

pub fn lower(t: &str) -> String {
    t.to_lowercase()
}

fn width(n: Int) -> usize {
    usize::try_from(n.max(0)).unwrap_or(usize::MAX)
}

pub fn take(t: &str, n: Int) -> String {
    t.chars().take(width(n)).collect()
}

pub fn drop(t: &str, n: Int) -> String {
    t.chars().skip(width(n)).collect()
}

pub fn split(t: &str, sep: &str) -> R<Vec<String>> {
    if sep.is_empty() {
        return Err(stop("E-TYPE-ARITY", "`split` takes one non-empty Text"));
    }
    Ok(t.split(sep).map(str::to_string).collect())
}

pub fn replace(t: &str, old: &str, new: &str) -> R<String> {
    if old.is_empty() {
        return Err(stop("E-TYPE-ARITY", "`replace` takes two Text, the first not empty"));
    }
    Ok(t.replace(old, new))
}

/// Text as source: quoted, with `\" \\ \n \t \{` and `\u{X}` for other control or white space but ` `.
pub fn quote(t: &str) -> String {
    let mut out = String::with_capacity(t.len() + 2);
    out.push('"');
    for c in t.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '{' => out.push_str("\\{"),
            c if c.is_control() || c.is_whitespace() && c != ' ' => out.push_str(&format!("\\u{{{:X}}}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
