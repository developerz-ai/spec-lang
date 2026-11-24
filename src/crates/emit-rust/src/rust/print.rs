//! Rust target trees to text: each item is parsed as `syn` (so a printer
//! bug is caught here, not by rustc) and printed with `prettyplease`.
//! Anchor statements `__slg!(n);` become `// slg: <file>:<line> <role>`
//! comments and start nested provenance spans.

use std::collections::BTreeMap;

use proc_macro2::TokenStream;
use spec_lang_diag::Diag;

use crate::provenance::{FileBuf, Origin};

fn bug(message: impl Into<String>) -> Diag {
    Diag::new(spec_lang_diag::Severity::Error, "E-UNSUPPORTED", message.into())
        .with_note("a printer produced Rust that does not parse: a backend bug, reported as unsupported")
}

/// Prints the items in `ts`; returns the text and where each anchor landed (line offset, origin).
pub fn items(ts: TokenStream, anchors: &[(String, Origin)]) -> Result<(String, Vec<(u32, Origin)>), Diag> {
    let file: syn::File = syn::parse2(ts.clone()).map_err(|e| bug(format!("{e}: {ts}")))?;
    let text = prettyplease::unparse(&file);
    let mut out = String::new();
    let mut nested = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let t = line.trim();
        if let Some(n) = t.strip_prefix("__slg!(").and_then(|r| r.strip_suffix(");"))
            && let Ok(n) = n.parse::<usize>()
            && let Some((comment, origin)) = anchors.get(n)
        {
            let indent = &line[..line.len() - line.trim_start().len()];
            out.push_str(indent);
            out.push_str(comment);
            out.push('\n');
            nested.push((i as u32, origin.clone()));
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    Ok((out, nested))
}

/// `use` lines for a set of paths, grouped by parent: `std` first, then `crate`/package, then `super`.
pub fn uses(paths: &std::collections::BTreeSet<String>) -> Result<String, Diag> {
    let mut groups: BTreeMap<(u8, String), Vec<String>> = BTreeMap::new();
    for p in paths {
        let (parent, last) = match p.rsplit_once("::") {
            Some((a, b)) => (a.to_string(), b.to_string()),
            None => continue,
        };
        let rank = if parent.starts_with("std") {
            0
        } else if parent.starts_with("super") {
            2
        } else {
            1
        };
        groups.entry((rank, parent)).or_default().push(last);
    }
    let mut out = String::new();
    let mut last_rank = None;
    for ((rank, parent), mut names) in groups {
        if last_rank.is_some_and(|r| r != rank) {
            out.push('\n');
        }
        last_rank = Some(rank);
        names.sort_by_key(|a| sort_key(a));
        names.dedup();
        let line = if names.len() == 1 {
            format!("use {parent}::{};\n", names[0])
        } else {
            format!("use {parent}::{{{}}};\n", names.join(", "))
        };
        // Print through prettyplease so long lists wrap as rustfmt would.
        let file: syn::File = syn::parse_str(&line).map_err(|e| bug(format!("{e}: {line}")))?;
        out.push_str(&prettyplease::unparse(&file));
    }
    Ok(out)
}

/// rustfmt's order inside braces: `self` first, then upper case before lower case.
fn sort_key(name: &str) -> (u8, String) {
    let first = name.chars().next().unwrap_or('a');
    let rank = if name == "self" {
        0
    } else if first.is_uppercase() {
        1
    } else {
        2
    };
    (rank, name.to_string())
}

/// A printed item: its text, origin, and nested spans (line offset, origin).
pub type Printed = (String, Origin, Vec<(u32, Origin)>);

/// A Rust source file: the provenance header, `use` lines, then items, one blank line apart.
pub struct RustFile {
    pub buf: FileBuf,
    header_origin: Origin,
    pending: Vec<Printed>,
}

impl RustFile {
    pub fn new(path: &str, header: &str, header_origin: Origin) -> Self {
        let mut buf = FileBuf::new(path);
        buf.push(&format!("// {header}\n"), header_origin.clone());
        RustFile { buf, header_origin, pending: Vec::new() }
    }

    /// Inner attributes or docs that must come before the `use` lines.
    pub fn prelude(&mut self, text: &str) {
        self.buf.push(text, self.header_origin.clone());
    }

    pub fn item(&mut self, text: String, origin: Origin, nested: Vec<(u32, Origin)>) {
        self.pending.push((text, origin, nested));
    }

    pub fn finish(mut self, uses: &str) -> FileBuf {
        if !uses.is_empty() {
            self.buf.push(&format!("\n{uses}"), self.header_origin.clone());
        }
        for (text, origin, nested) in self.pending {
            let nested = nested.into_iter().map(|(at, o)| (at + 1, o)).collect();
            self.buf.push_with(&format!("\n{text}"), origin, nested);
        }
        self.buf
    }
}
