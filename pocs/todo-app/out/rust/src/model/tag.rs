// Projected by hand from fn is_tag, type Tag (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

use std::fmt;

const MAX_SIZE: usize = 32;

/// `1 <= t.size <= 32 and t.bytes.all(97 <= it <= 122 or 48 <= it <= 57 or it == 45)
/// and not t.starts_with("-")`.
///
/// Every accepted byte is ASCII, so the size is the byte length and is checked first,
/// and the first code point is the first byte.
pub fn is_tag(text: &str) -> bool {
    let bytes = text.as_bytes();
    (1..=MAX_SIZE).contains(&bytes.len())
        && bytes[0] != b'-'
        && bytes
            .iter()
            .all(|&b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// `type Tag = Text where is_tag(self)`. Ordered by code point, which for a tag is byte order.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tag(Box<str>);

impl Tag {
    /// The claim is checked here and nowhere else; `None` is a value outside it.
    pub fn new(text: impl Into<Box<str>> + AsRef<str>) -> Option<Self> {
        is_tag(text.as_ref()).then(|| Self(text.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
