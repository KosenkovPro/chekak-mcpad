//! A backend that can be replaced while the server runs.
//!
//! The desktop shell opens the vault after the window appears, and it may open
//! another vault later. The server therefore does not hold a vault of its own,
//! it looks for the current one on every call.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use mcpad_app::{FileSlice, SearchHit, SearchOptions, VaultFile};
use mcpad_core::{Error, Result};

use crate::backend::Backend;

/// The backend of the vault that is open right now.
pub struct SwitchableBackend {
    inner: Mutex<Option<Box<dyn Backend>>>,
}

impl SwitchableBackend {
    /// Creates a backend without a vault.
    pub fn empty() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    /// Puts a vault behind the backend.
    pub fn set(&self, backend: Box<dyn Backend>) {
        *self.lock() = Some(backend);
    }

    /// Takes the vault away.
    pub fn clear(&self) {
        *self.lock() = None;
    }

    /// Whether a vault is open.
    pub fn has_vault(&self) -> bool {
        self.lock().is_some()
    }

    /// The backend of the current vault.
    fn current(&self) -> Result<MutexGuard<'_, Option<Box<dyn Backend>>>> {
        let guard = self.lock();
        if guard.is_none() {
            return Err(Error::message("no vault is open"));
        }
        Ok(guard)
    }

    /// Locks the backend and ignores a poisoned lock.
    fn lock(&self) -> MutexGuard<'_, Option<Box<dyn Backend>>> {
        self.inner.lock().unwrap_or_else(|error| error.into_inner())
    }
}

impl Backend for SwitchableBackend {
    fn root(&self) -> PathBuf {
        self.lock()
            .as_ref()
            .map(|backend| backend.root())
            .unwrap_or_default()
    }

    fn list_files(&self) -> Result<Vec<VaultFile>> {
        self.current()?
            .as_ref()
            .ok_or_else(|| Error::message("no vault is open"))?
            .list_files()
    }

    fn read_file(&self, relative: &str, start_line: usize, max_lines: usize) -> Result<FileSlice> {
        self.current()?
            .as_ref()
            .ok_or_else(|| Error::message("no vault is open"))?
            .read_file(relative, start_line, max_lines)
    }

    fn read_file_full(&self, relative: &str) -> Result<FileSlice> {
        self.current()?
            .as_ref()
            .ok_or_else(|| Error::message("no vault is open"))?
            .read_file_full(relative)
    }

    fn search(&self, query: &str, options: &SearchOptions) -> Result<Vec<SearchHit>> {
        self.current()?
            .as_ref()
            .ok_or_else(|| Error::message("no vault is open"))?
            .search(query, options)
    }

    fn write_file(
        &self,
        relative: &str,
        text: &str,
        encoding: Option<&str>,
        line_ending: Option<&str>,
    ) -> Result<()> {
        self.current()?
            .as_ref()
            .ok_or_else(|| Error::message("no vault is open"))?
            .write_file(relative, text, encoding, line_ending)
    }

    fn delete_file(&self, relative: &str) -> Result<()> {
        self.current()?
            .as_ref()
            .ok_or_else(|| Error::message("no vault is open"))?
            .delete_file(relative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::VaultBackend;

    fn vault_with_notes() -> (tempfile::TempDir, SwitchableBackend) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("notes")).unwrap();
        std::fs::write(dir.path().join("notes/a.md"), "# A\n").unwrap();
        let backend = SwitchableBackend::empty();
        backend.set(Box::new(VaultBackend::open(dir.path()).unwrap()));
        (dir, backend)
    }

    #[test]
    fn without_a_vault_the_tools_report_it() {
        let backend = SwitchableBackend::empty();
        let error = backend.list_files().unwrap_err();
        assert!(error.to_string().contains("no vault is open"));
        assert!(!backend.has_vault());
    }

    #[test]
    fn the_vault_can_be_replaced() {
        let first = tempfile::tempdir().unwrap();
        std::fs::write(first.path().join("a.md"), "first").unwrap();
        let second = tempfile::tempdir().unwrap();
        std::fs::write(second.path().join("b.md"), "second").unwrap();

        let backend = SwitchableBackend::empty();
        backend.set(Box::new(VaultBackend::open(first.path()).unwrap()));
        assert_eq!(backend.list_files().unwrap().len(), 1);

        backend.set(Box::new(VaultBackend::open(second.path()).unwrap()));
        let files = backend.list_files().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "b.md");
    }

    #[test]
    fn the_vault_can_be_taken_away() {
        let (_dir, backend) = vault_with_notes();
        assert!(backend.has_vault());
        backend.clear();
        assert!(!backend.has_vault());
        assert!(backend.list_files().is_err());
    }
}
