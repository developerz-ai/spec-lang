//! What an emitter hands back, and how a project lands on disk.

use crate::sem::prov::{self, Tracked};
use spec_lang_diag::{Diag, Severity};
use std::path::Path;

/// One file of a generated project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutFile {
    /// Relative to the project root, with `/`.
    pub path: String,
    pub bytes: Vec<u8>,
    pub executable: bool,
}

/// An emitter's error: the toolchain's one diagnostic shape.
pub type EmitError = Box<Diag>;

pub const PROVENANCE: &str = "provenance.json";

/// An error that is the backend's own fault, not the program's.
pub fn internal(what: impl std::fmt::Display) -> EmitError {
    Box::new(
        Diag::new(Severity::Error, "E-UNSUPPORTED", format!("unsupported: the backend could not finish ({what})"))
            .with_note("fix: this is a backend bug; report it with the program that caused it"),
    )
}

/// The files as bytes, sorted by path, with `provenance.json` computed from their text.
pub fn finish(target: &str, app: &str, mut files: Vec<Tracked>) -> Result<Vec<OutFile>, EmitError> {
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let provenance = prov::json(target, app, &files).map_err(internal)?;
    let mut out: Vec<OutFile> = files
        .into_iter()
        .map(|f| OutFile { path: f.path, bytes: f.text.into_bytes(), executable: f.executable })
        .collect();
    out.push(OutFile { path: PROVENANCE.to_string(), bytes: provenance.into_bytes(), executable: false });
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// Write a project under `dir`. A file that would be byte-identical is not rewritten.
pub fn write(dir: &Path, files: &[OutFile]) -> std::io::Result<()> {
    for f in files {
        let path = dir.join(&f.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if std::fs::read(&path).ok().as_deref() != Some(f.bytes.as_slice()) {
            std::fs::write(&path, &f.bytes)?;
        }
        #[cfg(unix)]
        if f.executable {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
        }
    }
    Ok(())
}

/// After the project's own formatter ran: read each file back and write the
/// provenance of the text as it is now.
pub fn reanchor(dir: &Path, target: &str, app: &str, files: &[Tracked]) -> Result<(), String> {
    let mut now = Vec::new();
    for f in files {
        let text = std::fs::read_to_string(dir.join(&f.path)).map_err(|e| format!("{}: {e}", f.path))?;
        now.push(Tracked { text, ..f.clone() });
    }
    now.sort_by(|a, b| a.path.cmp(&b.path));
    let json = prov::json(target, app, &now)?;
    std::fs::write(dir.join(PROVENANCE), json).map_err(|e| format!("{PROVENANCE}: {e}"))
}
