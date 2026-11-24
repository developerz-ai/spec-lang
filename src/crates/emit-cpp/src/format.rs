//! From a target tree to formatted text with provenance.
//!
//! A file's tree carries numbered markers (`// @@slg N`) where each chunk
//! starts, and readable anchors (`// slg: file:line role`) above guards. The
//! whole file is printed, formatted once by `clang-format`, then the markers
//! are removed; the line each marker stood on starts that chunk's span, and
//! each anchor starts a nested span that runs to the next anchor or the end of
//! its chunk.

use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};

use spec_lang_diag::Diag;
use spec_lang_emit_rust::provenance::{FileBuf, Origin, Span};

use crate::ast::{Decl, decl};

/// The style every generated file is formatted with; also written as the project's `.clang-format`.
pub const STYLE: &str = "BasedOnStyle: Google\nColumnLimit: 110\nReflowComments: false\nDerivePointerAlignment: false\nPointerAlignment: Left\n";

/// One C++ file being built.
#[derive(Debug)]
pub struct Unit {
    pub path: String,
    /// The first line: what the file came from.
    pub header: String,
    /// Project headers it includes; the standard headers are found from what the text uses.
    pub includes: Vec<String>,
    pub decls: Vec<Decl>,
    /// `origins[n]` is the origin of the chunk after marker `n`; `origins[0]` covers the header.
    origins: Vec<Origin>,
    anchors: HashMap<String, Origin>,
}

impl Unit {
    /// A file whose header (first line comment, includes) has `origin`.
    pub fn new(path: impl Into<String>, header: impl Into<String>, origin: Origin) -> Self {
        Unit {
            path: path.into(),
            header: header.into(),
            includes: Vec::new(),
            decls: Vec::new(),
            origins: vec![origin],
            anchors: HashMap::new(),
        }
    }

    pub fn include(&mut self, path: impl Into<String>) {
        let p = path.into();
        if !self.includes.contains(&p) {
            self.includes.push(p);
        }
    }

    /// A marker that starts a chunk with `origin`; place it before the chunk's declarations.
    pub fn mark(&mut self, origin: Origin) -> Decl {
        self.origins.push(origin);
        Decl::Marker((self.origins.len() - 1) as u32)
    }

    /// The text of a readable anchor comment for a nested span, registered with its origin.
    pub fn anchor(&mut self, origin: Origin) -> String {
        let text = anchor_text(&origin);
        self.anchors.insert(format!("// {text}"), origin);
        text
    }

    /// Prints, formats and returns the file with its spans.
    pub fn render(self) -> Result<FileBuf, Diag> {
        let mut body = String::new();
        for d in &self.decls {
            decl(d, &mut body);
        }
        let mut text = format!("// {}\n", self.header);
        if self.path.ends_with(".hpp") {
            text.push_str("#pragma once\n");
        }
        text.push('\n');
        for h in std_includes(&body) {
            text.push_str(&format!("#include <{h}>\n"));
        }
        text.push('\n');
        for h in &self.includes {
            text.push_str(&format!("#include \"{h}\"\n"));
        }
        text.push('\n');
        text.push_str(&body);
        let formatted = clang_format(&text, &self.path)?;
        let mut buf = FileBuf::new(self.path.clone());
        let mut chunk = String::new();
        let mut current = 0usize;
        let mut nested: Vec<(u32, Origin)> = Vec::new();
        let flush = |buf: &mut FileBuf, chunk: &mut String, current: usize, nested: &mut Vec<(u32, Origin)>| {
            if !chunk.is_empty() {
                buf.push_with(chunk, self.origins[current].clone(), std::mem::take(nested));
                chunk.clear();
            }
            nested.clear();
        };
        for line in formatted.lines() {
            let trimmed = line.trim();
            if let Some(n) = trimmed.strip_prefix("// @@slg ") {
                flush(&mut buf, &mut chunk, current, &mut nested);
                current = n.parse().unwrap_or(0).min(self.origins.len() - 1);
                continue;
            }
            if let Some(o) = self.anchors.get(trimmed) {
                nested.push((chunk.lines().count() as u32, o.clone()));
            }
            chunk.push_str(line);
            chunk.push('\n');
        }
        flush(&mut buf, &mut chunk, current, &mut nested);
        Ok(buf)
    }
}

/// `slg: src/todo/board.slg:46:8 guard[1]`.
pub fn anchor_text(origin: &Origin) -> String {
    format!("slg: {} {}", origin.slg, origin.role)
}

/// The standard headers a text uses, from the names it spells.
fn std_includes(text: &str) -> Vec<&'static str> {
    const TABLE: &[(&str, &str)] = &[
        ("std::ranges::", "algorithm"),
        ("std::array", "array"),
        ("std::int64_t", "cstdint"),
        ("std::uint8_t", "cstdint"),
        ("std::uint64_t", "cstdint"),
        ("std::size_t", "cstddef"),
        ("std::expected", "expected"),
        ("std::unexpected", "expected"),
        ("std::numeric_limits", "limits"),
        ("std::map", "map"),
        ("std::optional", "optional"),
        ("std::nullopt", "optional"),
        ("std::views::", "ranges"),
        ("std::set", "set"),
        ("std::span", "span"),
        ("std::string_view", "string_view"),
        ("std::string", "string"),
        ("std::tuple", "tuple"),
        ("std::pair", "utility"),
        ("std::move", "utility"),
        ("std::unreachable", "utility"),
        ("std::variant", "variant"),
        ("std::holds_alternative", "variant"),
        ("std::visit", "variant"),
        ("std::get", "variant"),
        ("std::vector", "vector"),
    ];
    let mut out: Vec<&'static str> = TABLE.iter().filter(|(n, _)| text.contains(n)).map(|(_, h)| *h).collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// Formats with the generated project's style. clang-format is part of this backend's toolchain.
pub fn clang_format(text: &str, path: &str) -> Result<String, Diag> {
    let style = format!("{{{}}}", STYLE.trim_end().replace('\n', ", "));
    let fail =
        |why: String| Diag::new(spec_lang_diag::Severity::Error, "E-LOAD", format!("clang-format on {path}: {why}"));
    let mut child = Command::new("clang-format")
        .arg(format!("--style={style}"))
        .arg(format!("--assume-filename={path}"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| fail(e.to_string()))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes()).map_err(|e| fail(e.to_string()))?;
    }
    let out = child.wait_with_output().map_err(|e| fail(e.to_string()))?;
    if !out.status.success() {
        return Err(fail(String::from_utf8_lossy(&out.stderr).into_owned()));
    }
    String::from_utf8(out.stdout).map_err(|e| fail(e.to_string()))
}

/// Spans of a file that is not formatted (a script, CMake, a runtime shim): one span.
pub fn whole(path: &str, text: &str, origin: Origin, executable: bool) -> FileBuf {
    let mut buf = FileBuf::new(path);
    if executable {
        buf = buf.executable();
    }
    buf.push(text, origin);
    buf
}

/// The spans and file of a finished buffer.
pub fn finish(buf: FileBuf) -> (spec_lang_emit_rust::OutFile, Vec<Span>) {
    buf.finish()
}
