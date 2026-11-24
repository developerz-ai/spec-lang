//! Text: code points, Unicode `White_Space` trimming, no normalisation.

use super::{Args, wrong_args};
use crate::error::Halted;
use crate::interp::Interp;
use crate::value::Value;
use spec_lang_syntax::Span;
use std::rc::Rc;

/// Unicode `White_Space`: `char::is_whitespace` is exactly that property
/// (25 code points; not U+FEFF, U+180E or U+200B).
pub(crate) fn trim(t: &str) -> &str {
    t.trim_matches(char::is_whitespace)
}

impl Interp<'_> {
    pub(crate) fn text_method(&mut self, t: Rc<str>, name: &str, args: &Args, span: Span) -> Halted<Option<Value>> {
        let texts: Vec<&str> =
            args.iter().filter_map(|(_, v)| if let Value::Text(s) = v { Some(&**s) } else { None }).collect();
        let n = match args.as_slice() {
            [(None, Value::Int(n))] => Some(*n),
            _ => None,
        };
        let count = |n: i128| usize::try_from(n.max(0)).unwrap_or(usize::MAX);
        Ok(Some(match (name, args.len()) {
            ("trim", 0) => Value::text(trim(&t)),
            ("lower", 0) => Value::text(&t.to_lowercase()),
            ("starts_with", 1) if texts.len() == 1 => Value::Bool(t.starts_with(texts[0])),
            ("contains", 1) if texts.len() == 1 => Value::Bool(t.contains(texts[0])),
            ("split", 1) if texts.len() == 1 && !texts[0].is_empty() => {
                Value::list(t.split(texts[0]).map(Value::text).collect())
            }
            ("replace", 2) if texts.len() == 2 && !texts[0].is_empty() => Value::text(&t.replace(texts[0], texts[1])),
            ("take", 1) if n.is_some() => Value::text(&t.chars().take(count(n.unwrap_or(0))).collect::<String>()),
            ("drop", 1) if n.is_some() => Value::text(&t.chars().skip(count(n.unwrap_or(0))).collect::<String>()),
            ("trim" | "lower", _) => return Err(wrong_args(span, name, "no arguments")),
            ("starts_with" | "contains" | "split", _) => return Err(wrong_args(span, name, "one non-empty Text")),
            ("replace", _) => return Err(wrong_args(span, name, "two Text, the first not empty")),
            _ => return Ok(None),
        }))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn trim_is_white_space_and_nothing_else() {
        let ws: Vec<char> = (0..=0x10FFFFu32).filter_map(char::from_u32).filter(|c| c.is_whitespace()).collect();
        assert_eq!(ws.len(), 25);
        assert_eq!(super::trim("\u{3000} a \u{85}"), "a");
        assert_eq!(super::trim("\u{FEFF}a\u{200B}"), "\u{FEFF}a\u{200B}");
        assert_eq!(super::trim("\u{180E}"), "\u{180E}");
    }
}
