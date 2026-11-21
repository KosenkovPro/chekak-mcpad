//! Filesystem helpers: atomic writes and recursive enumeration.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Error, Result};

static WRITE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Writes `bytes` to `path` atomically.
///
/// The payload is written to a temporary sibling file, flushed to disk and only
/// then moved over the destination. A crash therefore leaves either the old
/// content or the new content, never a truncated file.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| Error::message(format!("`{}` has no parent directory", path.display())))?;
    fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;

    let tmp = temp_sibling(path);

    {
        let mut file = fs::File::create(&tmp).map_err(|e| Error::io(&tmp, e))?;
        if let Err(e) = file.write_all(bytes) {
            let _ = fs::remove_file(&tmp);
            return Err(Error::io(&tmp, e));
        }
        if let Err(e) = file.sync_all() {
            let _ = fs::remove_file(&tmp);
            return Err(Error::io(&tmp, e));
        }
    }

    // Keep the permissions of the file we are replacing, if any.
    if let Ok(meta) = fs::metadata(path) {
        let _ = fs::set_permissions(&tmp, meta.permissions());
    }

    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(Error::io(path, e));
    }

    Ok(())
}

/// Builds a unique temporary file name next to `path`.
fn temp_sibling(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let seq = WRITE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!(".{name}.{nanos:x}{seq:x}.tmp"))
}

/// Recursively collects files below `root`.
///
/// `skip` is called for every directory; returning `true` prevents descending
/// into it. The result is sorted so callers get a stable order.
pub fn walk_files(root: &Path, skip: impl Fn(&Path) -> bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).map_err(|e| Error::io(&dir, e))?;
        for entry in entries {
            let entry = entry.map_err(|e| Error::io(&dir, e))?;
            let path = entry.path();
            let meta = entry.metadata().map_err(|e| Error::io(&path, e))?;
            if meta.is_dir() {
                if !skip(&path) {
                    stack.push(path);
                }
            } else if meta.is_file() {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

/// Ensures a directory exists.
pub fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|e| Error::io(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_creates_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("note.md");
        atomic_write(&target, b"hello").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"hello");
    }

    #[test]
    fn atomic_write_replaces_existing_content() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("note.md");
        atomic_write(&target, b"first").unwrap();
        atomic_write(&target, b"second").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"second");
    }

    #[test]
    fn atomic_write_leaves_no_temporary_files() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("note.md");
        atomic_write(&target, b"data").unwrap();
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "unexpected temp files: {leftovers:?}");
    }

    #[test]
    fn walk_files_skips_directories() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("notes")).unwrap();
        fs::create_dir_all(dir.path().join(".mcpad")).unwrap();
        fs::write(dir.path().join("notes/a.md"), b"a").unwrap();
        fs::write(dir.path().join(".mcpad/state.json"), b"{}").unwrap();

        let files = walk_files(dir.path(), |p| {
            p.file_name()
                .map(|n| n == std::ffi::OsStr::new(".mcpad"))
                .unwrap_or(false)
        })
        .unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("a.md"));
    }
}
