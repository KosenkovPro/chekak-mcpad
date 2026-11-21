//! A backend that works on a vault of the disk.

use std::path::{Path, PathBuf};

use mcpad_app::{FileSlice, SearchHit, SearchOptions, VaultFile, VaultService};
use mcpad_core::Result;

use crate::backend::Backend;

/// A backend on top of an open vault.
pub struct VaultBackend {
    service: VaultService,
}

impl VaultBackend {
    /// Opens a backend on a vault.
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            service: VaultService::open(root)?,
        })
    }

    /// The service behind the backend.
    pub fn service(&self) -> &VaultService {
        &self.service
    }
}

impl Backend for VaultBackend {
    fn root(&self) -> PathBuf {
        self.service.root().to_path_buf()
    }

    fn list_files(&self) -> Result<Vec<VaultFile>> {
        self.service.list_files()
    }

    fn read_file(&self, relative: &str, start_line: usize, max_lines: usize) -> Result<FileSlice> {
        self.service
            .read_file_range(relative, start_line, max_lines)
    }

    fn read_file_full(&self, relative: &str) -> Result<FileSlice> {
        let document = self.service.read_file_full(relative)?;
        let total_lines = document.text.lines().count();
        Ok(FileSlice {
            path: document.path,
            text: document.text,
            encoding: document.encoding,
            line_ending: document.line_ending,
            start_line: 1,
            returned_lines: total_lines,
            total_lines: Some(total_lines),
            truncated: false,
        })
    }

    fn search(&self, query: &str, options: &SearchOptions) -> Result<Vec<SearchHit>> {
        self.service.search(query, options)
    }

    fn write_file(
        &self,
        relative: &str,
        text: &str,
        encoding: Option<&str>,
        line_ending: Option<&str>,
    ) -> Result<()> {
        self.service
            .write_file(relative, text, encoding, line_ending, true)
    }

    fn delete_file(&self, relative: &str) -> Result<()> {
        self.service.delete_file(relative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault_with_notes() -> (tempfile::TempDir, VaultBackend) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("notes")).unwrap();
        std::fs::write(dir.path().join("notes/a.md"), "# A\n\nпервая\n").unwrap();
        let backend = VaultBackend::open(dir.path()).unwrap();
        (dir, backend)
    }

    #[test]
    fn the_backend_lists_the_files_of_the_vault() {
        let (_dir, backend) = vault_with_notes();
        let files = backend.list_files().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "notes/a.md");
    }

    #[test]
    fn the_backend_reads_a_range_of_lines() {
        let (_dir, backend) = vault_with_notes();
        let slice = backend.read_file("notes/a.md", 0, 2).unwrap();
        assert_eq!(slice.text, "# A\n\n");
        assert_eq!(slice.returned_lines, 2);
        assert_eq!(slice.total_lines, Some(3));
        assert!(slice.truncated);

        let slice = backend.read_file("notes/a.md", 2, 5).unwrap();
        assert_eq!(slice.text, "первая\n");
        assert_eq!(slice.start_line, 3);
        assert!(!slice.truncated);
    }

    #[test]
    fn the_backend_reads_a_file_completely() {
        let (_dir, backend) = vault_with_notes();
        let slice = backend.read_file_full("notes/a.md").unwrap();
        assert_eq!(slice.text, "# A\n\nпервая\n");
        assert!(!slice.truncated);
    }

    #[test]
    fn the_backend_writes_and_removes_a_file() {
        let (dir, backend) = vault_with_notes();
        backend
            .write_file("notes/b.md", "новая\n", None, None)
            .unwrap();
        assert!(dir.path().join("notes/b.md").exists());

        backend.delete_file("notes/b.md").unwrap();
        assert!(!dir.path().join("notes/b.md").exists());
    }

    #[test]
    fn the_backend_searches_the_vault() {
        let (_dir, backend) = vault_with_notes();
        let hits = backend.search("первая", &SearchOptions::default()).unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn the_backend_refuses_a_path_outside_of_the_vault() {
        let (_dir, backend) = vault_with_notes();
        assert!(backend.read_file("../secret.md", 0, 10).is_err());
    }
}
