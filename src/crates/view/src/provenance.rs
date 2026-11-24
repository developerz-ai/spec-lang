//! `provenance.json` of a generated project: which emitted lines came from
//! which declaration (`docs/idea/10-compiler-and-ir.md`, "Provenance").
//!
//! The shape read here is the one the emitters write:
//!
//! ```json
//! {"target": "bun", "app": "apps.bun.TodoBun",
//!  "spans": [{"file": "src/board/done.ts", "start": 1, "end": 12,
//!             "decl": "todo.board.done", "slg": "todo/board.slg:44:1",
//!             "role": "rule body", "strategy": "..."}]}
//! ```
//!
//! `file` and `decl` are required of every span; `start`, `end`, `role` and
//! `strategy` may be absent. Anything else in the file is ignored. A span
//! whose `decl` names no declaration of the program (a runtime shim) is kept
//! and simply never matches.

use crate::json::Json;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provenance {
    /// The backend's name: `rust`.
    pub backend: String,
    /// What a link to an emitted file starts with, relative to the site's
    /// folder: `../../../out/rust/`. Empty: files are named, not linked.
    pub href: String,
    pub spans: Vec<EmittedSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmittedSpan {
    /// Path inside the generated project.
    pub file: String,
    /// First and last line, 1-based.
    pub lines: Option<(i64, i64)>,
    /// What the span is: `rule body`, `guard[1]`; empty when not given.
    pub role: String,
    /// Qualified name of the declaration it was generated from.
    pub decl: String,
    pub strategy: Option<String>,
}

impl Provenance {
    /// Read the text of a `provenance.json`. `backend` is used when the file has no `target`.
    pub fn parse(text: &str, backend: &str, href: &str) -> Result<Provenance, String> {
        let json = Json::parse(text)?;
        let spans = json.get("spans").and_then(Json::as_arr).ok_or("provenance: no `spans` array")?;
        let mut out = Vec::new();
        for (i, s) in spans.iter().enumerate() {
            let field = |name: &str| {
                s.get(name)
                    .and_then(Json::as_str)
                    .map(str::to_string)
                    .ok_or(format!("provenance: span {i} has no `{name}`"))
            };
            let line = |name: &str| match s.get(name) {
                None => Ok(None),
                Some(v) => {
                    v.as_int().map(Some).ok_or(format!("provenance: span {i} has a `{name}` that is not an integer"))
                }
            };
            let lines = match (line("start")?, line("end")?) {
                (Some(lo), Some(hi)) => Some((lo, hi)),
                (Some(lo), None) => Some((lo, lo)),
                _ => None,
            };
            out.push(EmittedSpan {
                file: field("file")?,
                lines,
                role: s.get("role").and_then(Json::as_str).unwrap_or("").to_string(),
                decl: field("decl")?,
                strategy: s.get("strategy").and_then(Json::as_str).map(str::to_string),
            });
        }
        out.sort_by(|a, b| (&a.file, a.lines, &a.decl, &a.role).cmp(&(&b.file, b.lines, &b.decl, &b.role)));
        Ok(Provenance {
            backend: json.get("target").and_then(Json::as_str).unwrap_or(backend).to_string(),
            href: href.to_string(),
            spans: out,
        })
    }

    /// The spans generated from one declaration, in file then line order.
    pub fn of_decl<'a>(&'a self, qname: &'a str) -> impl Iterator<Item = &'a EmittedSpan> {
        self.spans.iter().filter(move |s| s.decl == qname)
    }
}

impl EmittedSpan {
    /// `src/board/done.rs:1-12`.
    pub fn text(&self) -> String {
        match self.lines {
            Some((lo, hi)) if lo == hi => format!("{}:{lo}", self.file),
            Some((lo, hi)) => format!("{}:{lo}-{hi}", self.file),
            None => self.file.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_spans_in_a_fixed_order() {
        let text = r#"{"target": "rust", "app": "m.App", "spans": [
            {"file": "src/b.rs", "decl": "m.b"},
            {"file": "src/a.rs", "start": 3, "end": 9, "decl": "m.a", "slg": "m.slg:1:1", "role": "rule body", "strategy": "rule.atomic"}]}"#;
        let p = Provenance::parse(text, "x", "../out/rust/").unwrap();
        assert_eq!(p.backend, "rust");
        assert_eq!(p.spans.iter().map(EmittedSpan::text).collect::<Vec<_>>(), ["src/a.rs:3-9", "src/b.rs"]);
        assert_eq!(p.of_decl("m.a").count(), 1);
    }

    #[test]
    fn refuses_a_span_without_a_declaration() {
        assert!(Provenance::parse(r#"{"spans": [{"file": "a.rs"}]}"#, "x", "").is_err());
        assert!(Provenance::parse(r#"{"files": []}"#, "x", "").is_err());
        assert!(Provenance::parse(r#"{"spans": [{"file": "a.rs", "decl": "m.a", "start": "1"}]}"#, "x", "").is_err());
    }
}
