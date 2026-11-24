//! Provenance: where every emitted line came from, in one step.
//!
//! A generated file is a sequence of **spans**. Each span starts at an
//! **anchor**: line 1 (the file header) or a comment holding
//! `slg: <file>:<line>:<col> <declaration> <role>`, and runs to the line before
//! the next anchor. So the spans tile the file, and they can be found again
//! after the target's own formatter has moved lines: [`spans`] reads the
//! anchors back out of the text. `provenance.json` is that table.

use spec_lang_diag::JsonWriter;
use spec_lang_ir::{DeclInfo, Program};
use spec_lang_syntax::Span;

/// The text every anchor comment starts with.
pub const MARK: &str = "slg: ";

/// What one span of a generated file is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The declaration, dotted: `todo.board.done`. Empty for a runtime shim.
    pub decl: String,
    /// `todo/board.slg:52:3`, or empty when no source line stands behind the span.
    pub slg: String,
    /// `rule`, `guard[1]`, `body.stmt[0]`, `cli table row`, `test`, `law`, `runtime shim: lock`, ...
    pub role: String,
    /// The strategy the emitter chose for this span.
    pub strategy: String,
    /// The fact in the IR that licensed the strategy.
    pub fact: String,
}

impl Entry {
    /// The comment text of this span's anchor.
    pub fn anchor(&self) -> String {
        format!("{MARK}{} {} {}", self.slg, self.decl, self.role)
    }
}

/// One emitted file with the spans it is made of, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tracked {
    /// Relative to the project root, with `/`.
    pub path: String,
    pub text: String,
    /// `entries[0]` starts at line 1; each later one at the next anchor comment.
    pub entries: Vec<Entry>,
    pub executable: bool,
}

/// One row of `provenance.json`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub file: String,
    pub start: usize,
    pub end: usize,
    pub entry: Entry,
}

/// `todo/board.slg:52:3` for a span; the file is named by its module, never by a local path.
pub fn slg(p: &Program, info: &DeclInfo, span: Span) -> String {
    let loc = p.location(span);
    format!("{}:{}:{}", super::names::slg_path(&p.module(info.module).name), loc.line, loc.col)
}

/// Find each entry's lines in the file's text as it is now.
pub fn spans(file: &Tracked) -> Result<Vec<Row>, String> {
    let lines: Vec<&str> = file.text.lines().collect();
    let mut starts = vec![1usize];
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.contains(MARK) {
            starts.push(i + 1);
        }
    }
    if starts.len() != file.entries.len() {
        return Err(format!(
            "{}: {} anchors in the text, {} spans recorded",
            file.path,
            starts.len(),
            file.entries.len()
        ));
    }
    let mut rows = Vec::new();
    for (i, entry) in file.entries.iter().enumerate() {
        let start = starts[i];
        let end = starts.get(i + 1).map_or(lines.len().max(1), |next| next - 1);
        if i > 0 && !lines[start - 1].contains(&entry.anchor()) {
            return Err(format!("{}:{start}: expected the anchor `{}`", file.path, entry.anchor()));
        }
        rows.push(Row { file: file.path.clone(), start, end, entry: entry.clone() });
    }
    Ok(rows)
}

/// `provenance.json`: one span per line, sorted by file and start line.
pub fn json(target: &str, app: &str, files: &[Tracked]) -> Result<String, String> {
    let mut rows = Vec::new();
    for f in files {
        rows.extend(spans(f)?);
    }
    rows.sort_by(|a, b| (&a.file, a.start).cmp(&(&b.file, b.start)));
    let mut out = String::from("{\"version\":1,\"generator\":\"spec-lang build\",");
    out.push_str(&format!("\"target\":{},\"app\":{},\"spans\":[\n", JsonWriter::str(target), JsonWriter::str(app)));
    for (i, r) in rows.iter().enumerate() {
        let row = JsonWriter::obj()
            .set("file", JsonWriter::str(&r.file))
            .set("start", JsonWriter::Int(r.start as i128))
            .set("end", JsonWriter::Int(r.end as i128))
            .set("decl", JsonWriter::str(&r.entry.decl))
            .set("slg", JsonWriter::str(&r.entry.slg))
            .set("role", JsonWriter::str(&r.entry.role))
            .set("strategy", JsonWriter::str(&r.entry.strategy))
            .set("fact", JsonWriter::str(&r.entry.fact));
        out.push_str(&row.to_string());
        out.push_str(if i + 1 < rows.len() { ",\n" } else { "\n" });
    }
    out.push_str("]}\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(role: &str) -> Entry {
        Entry { decl: "m.r".into(), slg: "m.slg:1:1".into(), role: role.into(), strategy: "s".into(), fact: "f".into() }
    }

    #[test]
    fn spans_tile_the_file_and_survive_reformatting() {
        let (a, b) = (entry("header"), entry("guard[0]"));
        let text = format!("// header\nx\n  // {}\ny\nz\n", b.anchor());
        let file = Tracked { path: "a.ts".into(), text, entries: vec![a, b.clone()], executable: false };
        let rows = spans(&file).unwrap();
        assert_eq!((rows[0].start, rows[0].end), (1, 2));
        assert_eq!((rows[1].start, rows[1].end), (3, 5));
        let moved = Tracked { text: format!("// header\n\nx\n\n// {}\ny\n", b.anchor()), ..file };
        let rows = spans(&moved).unwrap();
        assert_eq!((rows[1].start, rows[1].end), (5, 6));
    }

    #[test]
    fn a_missing_anchor_is_an_error() {
        let file = Tracked {
            path: "a.ts".into(),
            text: "x\n".into(),
            entries: vec![entry("a"), entry("b")],
            executable: false,
        };
        assert!(spans(&file).is_err());
    }
}
