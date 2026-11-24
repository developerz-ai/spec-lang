//! The two renderings of a [`Diag`].

use crate::{Diag, Edit, JsonWriter};
use spec_lang_syntax::{SourceMap, Span};

/// A file, and the line and column where a span starts and ends.
type Range = (String, (u32, u32), (u32, u32));

fn range(map: &SourceMap, span: Span) -> Option<Range> {
    let f = map.get(span.file)?;
    Some((f.path.clone(), f.line_col(span.lo), f.line_col(span.hi)))
}

fn where_text(map: &SourceMap, span: Span) -> String {
    match range(map, span) {
        Some((path, (l1, c1), (l2, c2))) => format!("{path}:{l1}:{c1}-{l2}:{c2}"),
        None => "<no source>".to_string(),
    }
}

fn edit_text(map: &SourceMap, e: &Edit) -> String {
    if e.span.lo == e.span.hi {
        format!("insert {:?} at {}", e.text, where_text(map, e.span))
    } else if e.text.is_empty() {
        format!("delete {}", where_text(map, e.span))
    } else {
        format!("replace {} with {:?}", where_text(map, e.span), e.text)
    }
}

impl Diag {
    /// rustc-style text: header, location with declaration path, the source
    /// line with carets, then `= ` lines for expected/found, edits and notes.
    pub fn human(&self, map: &SourceMap) -> String {
        let mut out = format!("{}[{}]: {}\n", self.severity.as_str(), self.code, self.message);
        let mut loc = match self.span {
            Some(s) => format!("  --> {}", where_text(map, s)),
            None => "  -->".to_string(),
        };
        if let Some(d) = &self.decl {
            loc.push_str(&format!("  in {d}"));
            if let Some(a) = &self.at {
                loc.push_str(&format!(" {a}"));
            }
        }
        out.push_str(&loc);
        out.push('\n');
        if let Some(s) = self.span {
            out.push_str(&snippet(map, s));
        }
        if let (Some(e), Some(f)) = (&self.expected, &self.found) {
            out.push_str(&format!("   = expected {e}, found {f}\n"));
        }
        for e in &self.edits {
            out.push_str(&format!("   = edit: {}\n", edit_text(map, e)));
        }
        for a in &self.alternatives {
            let edits: Vec<String> = a.edits.iter().map(|e| edit_text(map, e)).collect();
            out.push_str(&format!("   = option: {}: {}\n", a.does, edits.join("; ")));
        }
        for n in &self.notes {
            for (i, line) in n.lines().enumerate() {
                let lead = if i == 0 { "   = " } else { "     " };
                out.push_str(&format!("{lead}{line}\n"));
            }
        }
        out
    }

    /// One JSON object, the same facts as [`Diag::human`].
    pub fn json(&self, map: &SourceMap) -> JsonWriter {
        let pos = |(l, c): (u32, u32)| {
            JsonWriter::obj().set("line", JsonWriter::Int(l.into())).set("col", JsonWriter::Int(c.into()))
        };
        let span_json = |s: Span| {
            range(map, s).map(|(path, a, b)| {
                JsonWriter::obj().set("file", JsonWriter::str(path)).set("start", pos(a)).set("end", pos(b))
            })
        };
        let edit_json = |e: &Edit| {
            JsonWriter::obj()
                .set("at", span_json(e.span).unwrap_or(JsonWriter::Null))
                .set("text", JsonWriter::str(&e.text))
        };
        JsonWriter::obj()
            .set("severity", JsonWriter::str(self.severity.as_str()))
            .set("code", JsonWriter::str(self.code))
            .set("message", JsonWriter::str(&self.message))
            .set("span", self.span.and_then(span_json).unwrap_or(JsonWriter::Null))
            .set_opt("decl", self.decl.as_ref().map(JsonWriter::str))
            .set_opt("at", self.at.as_ref().map(JsonWriter::str))
            .set_opt("expected", self.expected.as_ref().map(JsonWriter::str))
            .set_opt("found", self.found.as_ref().map(JsonWriter::str))
            .set("edits", JsonWriter::Arr(self.edits.iter().map(edit_json).collect()))
            .set(
                "alternatives",
                JsonWriter::Arr(
                    self.alternatives
                        .iter()
                        .map(|a| {
                            JsonWriter::obj()
                                .set("does", JsonWriter::str(&a.does))
                                .set("edits", JsonWriter::Arr(a.edits.iter().map(edit_json).collect()))
                        })
                        .collect(),
                ),
            )
            .set("notes", JsonWriter::Arr(self.notes.iter().map(JsonWriter::str).collect()))
    }
}

/// The first line of the span with carets under it.
fn snippet(map: &SourceMap, span: Span) -> String {
    let Some(f) = map.get(span.file) else {
        return String::new();
    };
    let (line, col) = f.line_col(span.lo);
    let idx = (line - f.first_line) as usize;
    let Some(text) = f.text.lines().nth(idx) else {
        return String::new();
    };
    let start = col.saturating_sub(1) as usize;
    let (end_line, end_col) = f.line_col(span.hi);
    let end = if end_line == line {
        (end_col.saturating_sub(1) as usize).max(start + 1)
    } else {
        text.chars().count().max(start + 1)
    };
    let gutter = " ".repeat(line.to_string().len());
    format!("{gutter} |\n{line} | {text}\n{gutter} | {}{}\n", " ".repeat(start), "^".repeat(end - start))
}
