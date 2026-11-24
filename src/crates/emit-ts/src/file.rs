//! One TypeScript file being printed: its imports, gathered as names are used, and its lines.

use crate::names::relative;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default)]
struct Import {
    namespace: Option<String>,
    /// The namespace is read as a value somewhere, not only named in types.
    namespace_value: bool,
    values: BTreeSet<String>,
    types: BTreeSet<String>,
}

#[derive(Debug)]
pub struct TsFile {
    /// Relative to the project root.
    pub path: String,
    /// The first line: what the file is generated from.
    header: String,
    imports: BTreeMap<String, Import>,
    lines: Vec<String>,
    indent: usize,
}

impl TsFile {
    pub fn new(path: impl Into<String>, header: impl Into<String>) -> Self {
        TsFile { path: path.into(), header: header.into(), imports: BTreeMap::new(), lines: Vec::new(), indent: 0 }
    }

    /// `import * as alias from "<to>"`; `to` is a project path, or a package name.
    pub fn namespace(&mut self, to: &str, alias: &str) {
        let spec = self.spec(to);
        let imp = self.imports.entry(spec).or_default();
        imp.namespace = Some(alias.to_string());
        imp.namespace_value = true;
    }

    /// `import type * as alias from "<to>"`, unless the namespace is also read as a value.
    pub fn namespace_type(&mut self, to: &str, alias: &str) {
        let spec = self.spec(to);
        self.imports.entry(spec).or_default().namespace = Some(alias.to_string());
    }

    /// `import { name } from "<to>"`.
    pub fn value(&mut self, to: &str, name: &str) {
        let spec = self.spec(to);
        self.imports.entry(spec).or_default().values.insert(name.to_string());
    }

    /// `import type { name } from "<to>"`.
    pub fn type_only(&mut self, to: &str, name: &str) {
        let spec = self.spec(to);
        self.imports.entry(spec).or_default().types.insert(name.to_string());
    }

    /// The runtime, as `rt`.
    pub fn rt(&mut self) -> &'static str {
        self.namespace("src/runtime/index.ts", "rt");
        "rt"
    }

    /// The runtime, as `rt`, named in a type.
    pub fn rt_type(&mut self) -> &'static str {
        self.namespace_type("src/runtime/index.ts", "rt");
        "rt"
    }

    fn spec(&self, to: &str) -> String {
        if to.starts_with("src/") || to.starts_with("test/") { relative(&self.path, to) } else { to.to_string() }
    }

    pub fn line(&mut self, text: impl AsRef<str>) {
        let text = text.as_ref();
        if text.is_empty() {
            self.lines.push(String::new());
        } else {
            for l in text.split('\n') {
                self.lines.push(format!("{}{l}", "  ".repeat(self.indent)));
            }
        }
    }

    pub fn blank(&mut self) {
        if self.lines.last().is_some_and(|l| !l.is_empty()) {
            self.lines.push(String::new());
        }
    }

    /// A line ending in `{`, then the lines inside, indented.
    pub fn open(&mut self, text: impl AsRef<str>) {
        self.line(text);
        self.indent += 1;
    }

    /// Ends what [`TsFile::open`] began with this line (`}`, `});`, `} else {`).
    pub fn close(&mut self, text: impl AsRef<str>) {
        self.indent = self.indent.saturating_sub(1);
        self.line(text);
    }

    /// The file's text: header, imports in the order the project's formatter keeps them, then the body.
    pub fn finish(self) -> String {
        let mut out = String::new();
        for l in self.header.lines() {
            out.push_str("// ");
            out.push_str(l);
            out.push('\n');
        }
        let mut specs: Vec<(&String, &Import)> = self.imports.iter().collect();
        specs.sort_by_key(|(s, _)| import_rank(s));
        if !specs.is_empty() {
            out.push('\n');
        }
        for (spec, imp) in specs {
            if let Some(alias) = &imp.namespace {
                let kw = if imp.namespace_value { "import" } else { "import type" };
                out.push_str(&format!("{kw} * as {alias} from \"{spec}\";\n"));
            }
            if !imp.values.is_empty() || !imp.types.is_empty() {
                let mut names: Vec<(String, String)> = imp
                    .values
                    .iter()
                    .map(|v| (v.clone(), v.clone()))
                    .chain(imp.types.iter().filter(|t| !imp.values.contains(*t)).map(|t| (t.clone(), format!("type {t}"))))
                    .collect();
                names.sort_by(|a, b| natural(&a.0, &b.0));
                let all_types = imp.values.is_empty();
                let list: Vec<String> = names
                    .into_iter()
                    .map(|(n, shown)| if all_types { n } else { shown })
                    .collect();
                let kw = if all_types { "import type" } else { "import" };
                out.push_str(&format!("{kw} {{ {} }} from \"{spec}\";\n", list.join(", ")));
            }
        }
        out.push('\n');
        let mut body = self.lines;
        while body.last().is_some_and(String::is_empty) {
            body.pop();
        }
        for l in body {
            out.push_str(&l);
            out.push('\n');
        }
        out
    }
}

/// Packages before paths, farther paths before nearer ones, then by name: the formatter's order.
fn import_rank(spec: &str) -> (u8, usize, String) {
    if spec.contains(':') {
        return (0, 0, spec.to_string());
    }
    if !spec.starts_with('.') {
        return (1, 0, spec.to_string());
    }
    let ups = spec.matches("../").count();
    (2, usize::MAX - ups, spec.to_string())
}

/// Names compared as the formatter sorts them: case-insensitive first.
fn natural(a: &str, b: &str) -> std::cmp::Ordering {
    a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| a.cmp(b))
}
