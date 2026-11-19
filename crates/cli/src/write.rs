//! Writing files without ever leaving one half written.

use std::io::Write;
use std::path::Path;

use tempfile::NamedTempFile;

/// Replaces the file at `path` with `contents`: written to a temporary file next to it, given
/// the file's permissions and synced, then moved over it. On any failure the original is
/// untouched and the temporary file is removed.
// shortcut: the owner and group of the new file are the current user's, and other hard links to
// the old file keep the old text; preserve them if a user needs that.
pub fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temp = NamedTempFile::new_in(parent)?;
    temp.write_all(contents.as_bytes())?;
    temp.as_file()
        .set_permissions(std::fs::metadata(path)?.permissions())?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| e.error)?;
    Ok(())
}
