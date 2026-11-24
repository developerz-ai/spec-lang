// Projected by hand from fn is_title, type Title (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

use std::fmt;

use super::text::is_white_space;

const MAX_SIZE: usize = 200;
/// A code point is at most four bytes, so anything longer cannot be within `MAX_SIZE`.
const MAX_BYTES: usize = MAX_SIZE * 4;

/// `t.trim().size >= 1 and t.size <= 200 and t.bytes.all(it >= 32 and it != 127)
/// and not t.contains("\u{0085}") and not t.contains("\u{2028}") and not t.contains("\u{2029}")`.
///
/// The clauses are fused. On the ASCII path the byte claim has already
/// ruled out every white space but the space itself, so "not blank" is "some
/// byte is not a space" and the size is the byte length; the three line
/// breaks are not ASCII, so only the other path looks for them.
pub fn is_title(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() > MAX_BYTES || !bytes.iter().all(|&b| b >= 32 && b != 127) {
        return false;
    }
    if text.is_ascii() {
        return bytes.len() <= MAX_SIZE && bytes.iter().any(|&b| b != b' ');
    }
    let mut size = 0;
    let mut blank = true;
    for c in text.chars() {
        if is_line_break(c) {
            return false;
        }
        size += 1;
        blank = blank && is_white_space(c);
    }
    !blank && size <= MAX_SIZE
}

/// The line breaks above U+007F that a title refuses: next line, line separator, paragraph separator.
const fn is_line_break(c: char) -> bool {
    matches!(c, '\u{85}' | '\u{2028}' | '\u{2029}')
}

/// `type Title = Text where is_title(self)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Title(Box<str>);

impl Title {
    /// The claim is checked here and nowhere else; `None` is a value outside it.
    pub fn new(text: impl Into<Box<str>> + AsRef<str>) -> Option<Self> {
        is_title(text.as_ref()).then(|| Self(text.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Title {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
