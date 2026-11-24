//! Escaping for each output format, and identifiers that are safe in all of them.

use std::collections::BTreeSet;

/// Text inside a Mermaid label or message. Mermaid reads `#name;` and `#NN;`
/// as entities, so every character with a meaning in its grammar is sent as one.
pub fn mermaid(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '#' => out.push_str("#35;"),
            '"' => out.push_str("#quot;"),
            '<' => out.push_str("#lt;"),
            '>' => out.push_str("#gt;"),
            '&' => out.push_str("#amp;"),
            ';' => out.push_str("#59;"),
            '\\' => out.push_str("#92;"),
            '\n' | '\r' | '\t' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// Text inside a PlantUML label. `\n` would break the line and `"` would end
/// a quoted name, so both are sent as `<U+XXXX>`.
pub fn plantuml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("<U+005C>"),
            '"' => out.push_str("<U+0022>"),
            '\n' | '\r' | '\t' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// Text inside HTML, also inside a double-quoted attribute.
pub fn html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

/// Text inside a Markdown table cell or paragraph.
pub fn markdown(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '|' => out.push_str("\\|"),
            '<' => out.push_str("&lt;"),
            '\n' | '\r' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// Every run of white space as one space, ends trimmed: a source snippet on one line.
pub fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// At most `max` characters, the cut marked with `…`.
pub fn shorten(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Hands out identifiers of ASCII letters, digits and `_`, each one once.
/// A name that sanitises to one already taken gets `_2`, `_3`, ...
#[derive(Debug, Default)]
pub struct Ids {
    taken: BTreeSet<String>,
}

impl Ids {
    pub fn new() -> Self {
        Ids::default()
    }

    pub fn fresh(&mut self, name: &str) -> String {
        let mut base: String = name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        if base.is_empty() || base.starts_with(|c: char| c.is_ascii_digit()) {
            base.insert(0, 'n');
        }
        let mut id = base.clone();
        let mut n = 1;
        while !self.taken.insert(id.clone()) {
            n += 1;
            id = format!("{base}_{n}");
        }
        id
    }
}

/// A fenced Mermaid block for a Markdown file.
pub fn mermaid_block(body: &str) -> String {
    format!("```mermaid\n{}\n```\n", body.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mermaid_text_has_no_live_syntax() {
        assert_eq!(mermaid("a \"b\"; <c> #1 \\n"), "a #quot;b#quot;#59; #lt;c#gt; #35;1 #92;n");
    }

    #[test]
    fn ids_are_unique_and_plain() {
        let mut ids = Ids::new();
        assert_eq!(ids.fresh("todo.board.done"), "todo_board_done");
        assert_eq!(ids.fresh("todo.board_done"), "todo_board_done_2");
        assert_eq!(ids.fresh("3d"), "n3d");
    }

    #[test]
    fn shorten_marks_the_cut() {
        assert_eq!(shorten("abcdef", 4), "abc…");
        assert_eq!(shorten("abc", 4), "abc");
        assert_eq!(one_line(" a\n   b  "), "a b");
    }
}
