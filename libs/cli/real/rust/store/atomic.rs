//! An atomic replace after the rule: `<path>.<pid>.tmp` beside the store, `fsync`, `rename` over `<path>`.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

/// `<path><suffix>`: the lock and the temporary file live beside the store.
pub fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// A reader sees the old bytes or the new ones, never a mixture. On failure the temporary file is removed.
pub fn replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temporary = sibling(path, &format!(".{}.tmp", process::id()));
    let replaced = write_synced(&temporary, bytes).and_then(|()| fs::rename(&temporary, path));
    if replaced.is_err() {
        // The failure reported is the one above; a temporary that cannot be removed adds nothing to it.
        let _ = fs::remove_file(&temporary);
    }
    replaced
}

fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}
