//! Where every emitted line came from (docs/idea/10-compiler-and-ir.md,
//! Provenance): each span of an emitted file names its declaration, the
//! `.slg` location, its role, and the strategy with the fact that licensed it.
//! Written as `provenance.json` at the root of each generated project.

use spec_lang_diag::JsonWriter;

/// What a span of emitted lines carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin {
    /// `todo.board.done`, or `runtime` for a shim.
    pub decl: String,
    /// `src/todo/board.slg:44:1`; empty for a shim.
    pub slg: String,
    /// `rule body`, `guard[1]`, `cli param table row`, `runtime shim store.lock`, ...
    pub role: String,
    pub strategy: Option<String>,
    /// The IR fact that licensed the strategy.
    pub licence: Option<String>,
}

impl Origin {
    pub fn new(decl: impl Into<String>, slg: impl Into<String>, role: impl Into<String>) -> Self {
        Origin { decl: decl.into(), slg: slg.into(), role: role.into(), strategy: None, licence: None }
    }

    pub fn strategy(mut self, strategy: impl Into<String>, licence: impl Into<String>) -> Self {
        self.strategy = Some(strategy.into());
        self.licence = Some(licence.into());
        self
    }

    pub fn is_runtime(&self) -> bool {
        self.decl == "runtime"
    }
}

/// One span: lines `start..=end` (1-based) of one emitted file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub file: String,
    pub start: u32,
    pub end: u32,
    pub origin: Origin,
}

/// One emitted file: a relative path and its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutFile {
    pub path: String,
    pub bytes: Vec<u8>,
    /// Executable (`bin/check`).
    pub executable: bool,
}

/// A file being assembled from chunks, each with its origin.
#[derive(Debug)]
pub struct FileBuf {
    pub path: String,
    text: String,
    line: u32,
    pub spans: Vec<Span>,
    executable: bool,
}

impl FileBuf {
    pub fn new(path: impl Into<String>) -> Self {
        FileBuf { path: path.into(), text: String::new(), line: 1, spans: Vec::new(), executable: false }
    }

    pub fn executable(mut self) -> Self {
        self.executable = true;
        self
    }

    /// Appends `text` (whole lines) as one span.
    pub fn push(&mut self, text: &str, origin: Origin) {
        self.push_with(text, origin, Vec::new());
    }

    /// Appends `text` as one span, with nested spans that start at the given
    /// line offsets (0-based within `text`) and run to the next one or the end.
    pub fn push_with(&mut self, text: &str, origin: Origin, nested: Vec<(u32, Origin)>) {
        let text = if text.ends_with('\n') || text.is_empty() { text.to_string() } else { format!("{text}\n") };
        let lines = text.lines().count() as u32;
        if lines == 0 {
            return;
        }
        let start = self.line;
        let end = start + lines - 1;
        self.spans.push(Span { file: self.path.clone(), start, end, origin });
        for (i, (at, o)) in nested.iter().enumerate() {
            let s = start + at;
            let e = nested.get(i + 1).map_or(end, |(next, _)| start + next - 1);
            if s <= e {
                self.spans.push(Span { file: self.path.clone(), start: s, end: e, origin: o.clone() });
            }
        }
        self.text.push_str(&text);
        self.line = end + 1;
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn finish(self) -> (OutFile, Vec<Span>) {
        (OutFile { path: self.path, bytes: self.text.into_bytes(), executable: self.executable }, self.spans)
    }
}

/// The JSON document: spans sorted by file, start, end, then role; one per line.
pub fn to_json(backend: &str, app: &str, mut spans: Vec<Span>) -> String {
    spans.sort_by(|a, b| {
        (&a.file, a.start, std::cmp::Reverse(a.end), &a.origin.role).cmp(&(
            &b.file,
            b.start,
            std::cmp::Reverse(b.end),
            &b.origin.role,
        ))
    });
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"generator\": \"spec-lang build --target {backend}\",\n"));
    out.push_str(&format!("  \"app\": {},\n", JsonWriter::str(app)));
    out.push_str("  \"spans\": [\n");
    for (i, s) in spans.iter().enumerate() {
        let o = &s.origin;
        let row = JsonWriter::obj()
            .set("file", JsonWriter::str(&s.file))
            .set("start", JsonWriter::Int(i128::from(s.start)))
            .set("end", JsonWriter::Int(i128::from(s.end)))
            .set("decl", JsonWriter::str(&o.decl))
            .set("slg", JsonWriter::str(&o.slg))
            .set("role", JsonWriter::str(&o.role))
            .set_opt("strategy", o.strategy.as_ref().map(JsonWriter::str))
            .set_opt("licence", o.licence.as_ref().map(JsonWriter::str));
        out.push_str("    ");
        out.push_str(&row.to_string());
        out.push_str(if i + 1 < spans.len() { ",\n" } else { "\n" });
    }
    out.push_str("  ]\n}\n");
    out
}
