// Projected by hand from Text.size and Text.trim as fixed by docs/contract.md section 1, used by fn is_title (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

/// Unicode `White_Space`: exactly the 25 code points the contract lists.
/// Written out so the set cannot move with the toolchain's Unicode tables.
pub const fn is_white_space(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'..='\u{000D}'
            | '\u{0020}'
            | '\u{0085}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
    )
}

/// `Text.trim`: both ends, `White_Space` only.
pub fn trim(text: &str) -> &str {
    text.trim_matches(is_white_space)
}

/// `Text.size`: code points, not bytes and not UTF-16 units.
pub fn size(text: &str) -> usize {
    if text.is_ascii() {
        text.len()
    } else {
        text.chars().count()
    }
}
