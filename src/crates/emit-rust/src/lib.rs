//! The Rust backend of `spec-lang build`.
//!
//! [`emit`] turns one app of a resolved program into a complete Cargo project:
//! the program's types, fns, rules, `cli` table, store codec and tests,
//! generated as Rust target trees (`syn`) and printed with `prettyplease`,
//! plus the fixed runtime shims in `runtime/`, emitted verbatim, and
//! `provenance.json`. The [`front`] (typing, facts, the derived `cli` table)
//! is shared with the C++ backend until it moves into the IR crate.

pub mod front;
pub mod provenance;

pub use provenance::OutFile;
pub mod rust;

use spec_lang_diag::Diag;
use spec_lang_ir::{AppDecl, AppId, Program};

/// Every strategy a backend may choose, by name. Each has a plain translation
/// it falls back to when switched off (docs/idea/37-optimizer.md). Not here:
/// a read-only rule's shared lock and its never writing the store, which the
/// contract requires of every translation (`docs/contract.md` section 3).
pub const STRATEGIES: &[&str] = &[
    "entry.lookup",
    "entry.remove",
    "filter-remove.retain",
    "store.read.projection",
    "sort.unstable",
    "sink.inline-format",
    "text.inline",
    "link.static",
];

/// Strategies off unless asked for: each measured and not a clear win.
/// `text.inline` (a refined text held in place, no heap): on the 100,000-task
/// store it cut `add`'s allocations by a quarter but raised its peak RSS from
/// 55.6 to 64.0 MB, since every set node then holds the full inline width.
pub const OFF_BY_DEFAULT: &[&str] = &["text.inline"];

/// What `emit` may do beyond the plain translation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// Strategies switched off, by name from [`STRATEGIES`].
    pub off: std::collections::BTreeSet<String>,
}

impl Default for Options {
    /// Every strategy that measured as a win.
    fn default() -> Self {
        Options { off: OFF_BY_DEFAULT.iter().map(|s| s.to_string()).collect() }
    }
}

impl Options {
    /// Every strategy on, those off by default included.
    pub fn all() -> Self {
        Options { off: std::collections::BTreeSet::new() }
    }

    /// Every strategy off: the plain translation.
    pub fn plain() -> Self {
        Options { off: STRATEGIES.iter().map(|s| s.to_string()).collect() }
    }
}

/// Emits one app as a Cargo project with every strategy on.
pub fn emit(program: &Program, app: &AppDecl) -> Result<Vec<OutFile>, Vec<Diag>> {
    emit_with(program, app, &Options::default())
}

/// Emits one app as a Cargo project: relative paths and bytes, sorted by path.
/// Same program and options, same bytes.
pub fn emit_with(program: &Program, app: &AppDecl, options: &Options) -> Result<Vec<OutFile>, Vec<Diag>> {
    if let Some(unknown) = options.off.iter().find(|s| !STRATEGIES.contains(&s.as_str())) {
        return Err(vec![Diag::load(format!("unknown strategy {unknown}; known: {}", STRATEGIES.join(", ")))]);
    }
    let Some(id) = program.apps.iter().position(|a| std::ptr::eq(a, app) || a.info == app.info) else {
        return Err(vec![Diag::load(format!("app {} is not in the program", app.info.name))]);
    };
    let front = front::build(program, AppId(id as u32))?;
    rust::emit(&front, options)
}

/// Writes emitted files under `dir`, replacing only files whose bytes changed
/// (a byte-identical file is not rewritten: docs/idea/32-build-layout.md).
pub fn write(dir: &std::path::Path, files: &[OutFile]) -> std::io::Result<()> {
    for f in files {
        let path = dir.join(&f.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if std::fs::read(&path).ok().as_deref() != Some(f.bytes.as_slice()) {
            std::fs::write(&path, &f.bytes)?;
        }
        if f.executable {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
        }
    }
    Ok(())
}
