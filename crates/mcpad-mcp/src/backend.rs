//! What the server can do with a vault.
//!
//! The trait keeps the protocol free of the disk: the same server serves a
//! vault of the disk, a vault of a test, and any other store a client may
//! need later.

use std::path::PathBuf;

use mcpad_app::{FileSlice, SearchHit, SearchOptions, VaultFile};
use mcpad_core::Result;

/// The number of lines a read returns when the client asks for nothing else.
pub const DEFAULT_READ_LINES: usize = 2000;

/// The operations a backend has to provide.
pub trait Backend: Send + Sync {
    /// The root of the vault.
    fn root(&self) -> PathBuf;

    /// Every file of the vault.
    fn list_files(&self) -> Result<Vec<VaultFile>>;

    /// Reads a range of lines of a file.
    ///
    /// The first line is counted from zero, the way the disk counts it.
    fn read_file(&self, relative: &str, start_line: usize, max_lines: usize) -> Result<FileSlice>;

    /// Reads a file completely.
    fn read_file_full(&self, relative: &str) -> Result<FileSlice>;

    /// Searches the text of the vault.
    fn search(&self, query: &str, options: &SearchOptions) -> Result<Vec<SearchHit>>;

    /// Writes a file of the vault.
    fn write_file(
        &self,
        relative: &str,
        text: &str,
        encoding: Option<&str>,
        line_ending: Option<&str>,
    ) -> Result<()>;

    /// Removes a file of the vault.
    fn delete_file(&self, relative: &str) -> Result<()>;
}

/// A backend behind a shared pointer works as the backend itself.
///
/// The desktop shell keeps the pointer to put another vault behind the server
/// without stopping it.
impl<T: Backend + ?Sized> Backend for std::sync::Arc<T> {
    fn root(&self) -> PathBuf {
        (**self).root()
    }

    fn list_files(&self) -> Result<Vec<VaultFile>> {
        (**self).list_files()
    }

    fn read_file(&self, relative: &str, start_line: usize, max_lines: usize) -> Result<FileSlice> {
        (**self).read_file(relative, start_line, max_lines)
    }

    fn read_file_full(&self, relative: &str) -> Result<FileSlice> {
        (**self).read_file_full(relative)
    }

    fn search(&self, query: &str, options: &SearchOptions) -> Result<Vec<SearchHit>> {
        (**self).search(query, options)
    }

    fn write_file(
        &self,
        relative: &str,
        text: &str,
        encoding: Option<&str>,
        line_ending: Option<&str>,
    ) -> Result<()> {
        (**self).write_file(relative, text, encoding, line_ending)
    }

    fn delete_file(&self, relative: &str) -> Result<()> {
        (**self).delete_file(relative)
    }
}
