//! Character classes, matched the way the reference checker's regexes match them.

use crate::unicode_tables::{DIGIT, SPACE, WORD};

fn in_table(table: &[(u32, u32)], c: char) -> bool {
    let c = c as u32;
    table
        .binary_search_by(|&(lo, hi)| {
            if hi < c {
                std::cmp::Ordering::Less
            } else if lo > c {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// Python `\s`: what the grammar's `%ignore /\s+/` skips.
pub(crate) fn is_space(c: char) -> bool {
    in_table(SPACE, c)
}

fn is_superscript(c: char) -> bool {
    c == '²' || c == '³'
}

/// `NAME` start: `[^\W\d²³]`.
pub(crate) fn is_name_start(c: char) -> bool {
    in_table(WORD, c) && !in_table(DIGIT, c) && !is_superscript(c)
}

/// `NAME` continuation: `(?:(?![²³])\w)`.
pub(crate) fn is_name_continue(c: char) -> bool {
    in_table(WORD, c) && !is_superscript(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_follow_python() {
        assert!(is_name_start('_'));
        assert!(is_name_start('µ'));
        assert!(is_name_start('½'), "No category is \\w and not \\d");
        assert!(!is_name_start('7'));
        assert!(!is_name_start('²'));
        assert!(is_name_continue('7'));
        assert!(!is_name_continue('³'));
        assert!(is_space('\u{1c}'), "Python counts the information separators as space");
        assert!(is_space('\u{3000}'));
        assert!(!is_space('\u{200b}'));
    }
}
