//! `slg` code fences in Markdown, found exactly as the reference checker finds
//! them: the regex `^```slg\n(.*?)^```` in multi-line, dot-all mode.

/// One fenced block: its text and the line number of its first line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fence {
    pub body: String,
    pub first_line: u32,
}

/// Every `slg` fence of a Markdown text, in order. A fence with no closing
/// line is not a fence.
pub fn slg_fences(markdown: &str) -> Vec<Fence> {
    let mut out = Vec::new();
    let mut search = 0;
    while let Some(open) = find_line_start(markdown, search, "```slg\n") {
        let body_start = open + "```slg\n".len();
        let Some(close) = find_line_start(markdown, body_start, "```") else {
            break;
        };
        let body = markdown.get(body_start..close).unwrap_or("").to_string();
        let line = markdown.get(..open).map_or(0, |s| s.matches('\n').count());
        out.push(Fence { body, first_line: crate::span::to_u32(line + 2) });
        search = close + 3;
    }
    out
}

/// The first offset `>= from` at the start of a line where `needle` begins.
fn find_line_start(text: &str, from: usize, needle: &str) -> Option<usize> {
    let mut i = from;
    while i <= text.len() {
        let at_line_start = i == 0 || text.as_bytes().get(i - 1) == Some(&b'\n');
        if at_line_start && text.get(i..).is_some_and(|s| s.starts_with(needle)) {
            return Some(i);
        }
        i = i + text.get(i..)?.find('\n')? + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_fences_and_their_lines() {
        let md = "# t\n```slg\na b;\n```\ntext\n```text\nno\n```\n```slg\n```\n";
        let f = slg_fences(md);
        assert_eq!(f.len(), 2);
        assert_eq!(f[0], Fence { body: "a b;\n".into(), first_line: 3 });
        assert_eq!(f[1].body, "");
    }

    #[test]
    fn indented_or_unclosed_fences_do_not_count() {
        assert!(slg_fences("  ```slg\na b;\n```\n").is_empty());
        assert!(slg_fences("```slg\na b;\n").is_empty());
        assert!(slg_fences("```slg x\na b;\n```\n").is_empty());
    }
}
