// Projected by hand from note @store: an atomic replace after the rule (docs/cli-vocabulary.md, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::process;

use super::path::sibling;

/// Write `<path>.<pid>.tmp` beside the store, `fsync`, `rename` over `<path>`.
/// A reader sees the old bytes or the new ones, never a mixture.
pub(super) fn replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temporary = sibling(path, &format!(".{}.tmp", process::id()));
    let replaced = write_synced(&temporary, bytes).and_then(|()| fs::rename(&temporary, path));
    if replaced.is_err() {
        // The failure reported is the one above; a temporary that cannot be removed adds nothing to it.
        let _ = fs::remove_file(&temporary);
    }
    replaced
}

/// The bytes arrive as one buffer, so this is one `write` and one `fsync`.
fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}
