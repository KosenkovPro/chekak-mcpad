//! The application service layer of MCPad.
//!
//! Everything that the desktop shell, the command line and the MCP server do
//! with a vault goes through [`VaultService`]. The Tauri commands are thin
//! wrappers around it, so the behaviour can be tested without a window.

use std::path::{Component, Path, PathBuf};

use mcpad_core::session::{Session, SessionStore};
use mcpad_core::{Error, Result, Theme, Vault};
use mcpad_text::{Document, EncodingKind, LargeFile, LineEnding};
use serde::Serialize;

pub mod recent;
pub mod search;

pub use recent::{last_vault, remember_vault};
pub use search::{SearchHit, SearchOptions};

/// Files above this size are opened lazily instead of being loaded completely.
pub const LARGE_FILE_THRESHOLD: u64 = 8 * 1024 * 1024;

/// The largest file that may be loaded completely into the editor.
pub const FULL_LOAD_LIMIT: u64 = 256 * 1024 * 1024;

/// A file of the vault.
#[derive(Debug, Clone, Serialize)]
pub struct VaultFile {
    /// Path relative to the vault root.
    pub path: String,
    /// File name including the extension.
    pub name: String,
    /// `note`, `canvas` or `other`.
    pub kind: String,
    /// Size in bytes.
    pub size: u64,
}

/// The content of a file, ready for the editor.
#[derive(Debug, Clone, Serialize)]
pub struct OpenDocument {
    /// Path relative to the vault root.
    pub path: String,
    /// Content of the file.
    pub text: String,
    /// Encoding label.
    pub encoding: String,
    /// Line ending label.
    pub line_ending: String,
    /// Whether the file is opened in lazy mode.
    pub large: bool,
}

/// A part of a file.
#[derive(Debug, Clone, Serialize)]
pub struct FileSlice {
    /// Path relative to the vault root.
    pub path: String,
    /// The text of the part.
    pub text: String,
    /// Encoding label.
    pub encoding: String,
    /// Line ending label.
    pub line_ending: String,
    /// The first line of the part, counted from one.
    pub start_line: usize,
    /// The number of lines in the part.
    pub returned_lines: usize,
    /// The number of lines in the whole file, when it is known.
    pub total_lines: Option<usize>,
    /// Whether the file holds more lines than the part.
    pub truncated: bool,
}

/// An open vault.
#[derive(Debug, Clone)]
pub struct VaultService {
    vault: Vault,
    root: PathBuf,
}

impl VaultService {
    /// Opens a vault.
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let vault = Vault::open(&root)?;
        Ok(Self { vault, root })
    }

    /// The vault root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The vault itself.
    pub fn vault(&self) -> &Vault {
        &self.vault
    }

    /// The store of the persisted session.
    pub fn session_store(&self) -> SessionStore {
        SessionStore::new(self.vault.layout().clone())
    }

    /// Lists the files of the vault.
    pub fn list_files(&self) -> Result<Vec<VaultFile>> {
        let mut files = Vec::new();
        for path in self.vault.syncable_files()? {
            let size = std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
            files.push(VaultFile {
                path: relative_of(&self.root, &path),
                name: path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                kind: kind_of(&path).to_string(),
                size,
            });
        }
        Ok(files)
    }

    /// Reads a file of the vault.
    pub fn read_file(&self, relative: &str) -> Result<OpenDocument> {
        let absolute = self.resolve(relative)?;
        let metadata = std::fs::metadata(&absolute).map_err(|e| Error::io(&absolute, e))?;
        let large = metadata.len() > LARGE_FILE_THRESHOLD;

        if large {
            let file = LargeFile::open(&absolute)?;
            let text = file.read_lines(0, 5000)?;
            return Ok(OpenDocument {
                path: relative.to_string(),
                text,
                encoding: file.encoding().label().to_string(),
                line_ending: "LF".to_string(),
                large,
            });
        }

        let bytes = std::fs::read(&absolute).map_err(|e| Error::io(&absolute, e))?;
        let document = Document::from_bytes(&bytes);

        Ok(OpenDocument {
            path: relative.to_string(),
            text: document.text(),
            encoding: document.encoding().label().to_string(),
            line_ending: document.line_ending().label().to_string(),
            large,
        })
    }

    /// Reads a file of the vault completely, no matter how big it is.
    ///
    /// A huge file is opened this way only when the user asks for it, so the
    /// editor never holds a truncated text that could be saved back by mistake.
    pub fn read_file_full(&self, relative: &str) -> Result<OpenDocument> {
        let absolute = self.resolve(relative)?;
        let metadata = std::fs::metadata(&absolute).map_err(|e| Error::io(&absolute, e))?;

        if metadata.len() > FULL_LOAD_LIMIT {
            return Err(Error::Config(format!(
                "`{}` is bigger than {} MB and does not fit into memory",
                relative,
                FULL_LOAD_LIMIT / 1024 / 1024
            )));
        }

        let bytes = std::fs::read(&absolute).map_err(|e| Error::io(&absolute, e))?;
        let document = Document::from_bytes(&bytes);

        Ok(OpenDocument {
            path: relative.to_string(),
            text: document.text(),
            encoding: document.encoding().label().to_string(),
            line_ending: document.line_ending().label().to_string(),
            large: false,
        })
    }

    /// Writes a file of the vault.
    ///
    /// The content is encoded with the requested encoding and line ending and
    /// written atomically, so a crash cannot leave a half written file behind.
    ///
    /// `complete` tells whether the text is the whole content of the file. A
    /// part of a huge file must never be written back, because that would cut
    /// the rest of the note away.
    pub fn write_file(
        &self,
        relative: &str,
        text: &str,
        encoding: Option<&str>,
        line_ending: Option<&str>,
        complete: bool,
    ) -> Result<()> {
        let absolute = self.resolve(relative)?;
        self.refuse_service_path(relative)?;

        if !complete {
            let metadata = std::fs::metadata(&absolute).map_err(|e| Error::io(&absolute, e))?;
            if metadata.len() > LARGE_FILE_THRESHOLD {
                return Err(Error::Config(format!(
                    "`{relative}` is open partially, load it completely before saving"
                )));
            }
        }

        let encoding = match encoding {
            Some(label) => EncodingKind::from_label(label)
                .ok_or_else(|| Error::Config(format!("unknown encoding `{label}`")))?,
            None => detect_encoding(&absolute),
        };

        let line_ending = match line_ending {
            Some(label) => LineEnding::from_label(label)
                .ok_or_else(|| Error::Config(format!("unknown line ending `{label}`")))?,
            None => detect_line_ending(&absolute),
        };

        let bytes = encoding.encode(&LineEnding::convert(text, line_ending));
        mcpad_core::fs::atomic_write(&absolute, &bytes)
    }

    /// Converts a file to another encoding or line ending, in place.
    pub fn convert_file(
        &self,
        relative: &str,
        encoding: Option<&str>,
        line_ending: Option<&str>,
    ) -> Result<()> {
        let absolute = self.resolve(relative)?;
        let bytes = std::fs::read(&absolute).map_err(|e| Error::io(&absolute, e))?;
        let document = Document::from_bytes(&bytes);

        let target_encoding = match encoding {
            Some(label) => EncodingKind::from_label(label)
                .ok_or_else(|| Error::Config(format!("unknown encoding `{label}`")))?,
            None => document.encoding(),
        };

        let target_line_ending = match line_ending {
            Some(label) => LineEnding::from_label(label)
                .ok_or_else(|| Error::Config(format!("unknown line ending `{label}`")))?,
            None => document.line_ending(),
        };

        let text = LineEnding::convert(&document.text(), target_line_ending);
        let bytes = target_encoding.encode(&text);
        mcpad_core::fs::atomic_write(&absolute, &bytes)
    }

    /// Reads a range of lines of a file.
    ///
    /// A huge file is read window by window, so a client can look at a part of
    /// a log without loading all of it.
    pub fn read_file_range(
        &self,
        relative: &str,
        start_line: usize,
        max_lines: usize,
    ) -> Result<FileSlice> {
        let absolute = self.resolve(relative)?;
        let metadata = std::fs::metadata(&absolute).map_err(|e| Error::io(&absolute, e))?;

        if metadata.len() > LARGE_FILE_THRESHOLD {
            let file = LargeFile::open(&absolute)?;
            let text = file.read_lines(start_line, max_lines)?;
            let returned_lines = text.lines().count();
            let total_lines = file.line_count();
            return Ok(FileSlice {
                path: relative.to_string(),
                encoding: file.encoding().label().to_string(),
                line_ending: "LF".to_string(),
                start_line: start_line + 1,
                returned_lines,
                total_lines: Some(total_lines),
                truncated: start_line + returned_lines < total_lines,
                text,
            });
        }

        let bytes = std::fs::read(&absolute).map_err(|e| Error::io(&absolute, e))?;
        let document = Document::from_bytes(&bytes);
        let all = document.text();

        // The line endings of the file are kept, only whole lines are cut out.
        let lines: Vec<&str> = all.split_inclusive('\n').collect();
        let total_lines = lines.len();
        let start = start_line.min(total_lines);
        let end = start.saturating_add(max_lines).min(total_lines);

        Ok(FileSlice {
            path: relative.to_string(),
            text: lines[start..end].concat(),
            encoding: document.encoding().label().to_string(),
            line_ending: document.line_ending().label().to_string(),
            start_line: start + 1,
            returned_lines: end - start,
            total_lines: Some(total_lines),
            truncated: end < total_lines,
        })
    }

    /// Searches the text of every file of the vault.
    pub fn search(&self, query: &str, options: &SearchOptions) -> Result<Vec<SearchHit>> {
        let files = self.vault.syncable_files()?;
        search::search_files(&self.root, &files, query, options)
    }

    /// Removes a file of the vault.
    ///
    /// Only files are removed, never directories, and never the service
    /// directory with the session and the drafts.
    pub fn delete_file(&self, relative: &str) -> Result<()> {
        let absolute = self.resolve(relative)?;
        self.refuse_service_path(relative)?;

        let metadata = std::fs::metadata(&absolute).map_err(|e| Error::io(&absolute, e))?;
        if metadata.is_dir() {
            return Err(Error::Config(format!(
                "`{relative}` is a directory, remove its files one by one"
            )));
        }

        std::fs::remove_file(&absolute).map_err(|e| Error::io(&absolute, e))
    }

    /// Reads the persisted session.
    pub fn load_session(&self) -> Session {
        self.session_store().load()
    }

    /// Writes the persisted session.
    pub fn save_session(&self, session: &Session) -> Result<()> {
        self.session_store().save(session)
    }

    /// Stores the unsaved content of a file.
    pub fn save_draft(&self, relative: &str, text: &str) -> Result<()> {
        self.session_store().save_draft(relative, text)
    }

    /// Reads the unsaved content of a file.
    pub fn load_draft(&self, relative: &str) -> Option<String> {
        self.session_store().load_draft(relative)
    }

    /// Removes the unsaved content of a file.
    pub fn discard_draft(&self, relative: &str) -> Result<()> {
        self.session_store().discard_draft(relative)
    }

    /// The absolute path of a vault relative path.
    ///
    /// A path that points outside of the vault is rejected, so a tool that
    /// writes files can never touch the rest of the disk.
    fn resolve(&self, relative: &str) -> Result<PathBuf> {
        let candidate = Path::new(relative);
        if candidate.is_absolute() {
            return Err(Error::Config(format!(
                "`{relative}` must be a path inside the vault"
            )));
        }

        let mut resolved = self.root.clone();
        for component in candidate.components() {
            match component {
                Component::CurDir => {}
                Component::Normal(part) => resolved.push(part),
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(Error::Config(format!(
                        "`{relative}` points outside of the vault"
                    )));
                }
            }
        }

        Ok(resolved)
    }

    /// Refuses to change a file of the service directory.
    fn refuse_service_path(&self, relative: &str) -> Result<()> {
        let path = Path::new(relative);
        if self.vault.is_service_path(path) {
            return Err(Error::Config(format!(
                "`{relative}` belongs to the service directory of MCPad"
            )));
        }
        Ok(())
    }
}

/// Loads a theme from the directory with the built-in themes.
pub fn load_theme(dir: &Path, name: &str) -> Result<Theme> {
    Theme::load(&dir.join(format!("{name}.json")))
}

/// Turns an absolute path into a vault relative path with forward slashes.
fn relative_of(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn kind_of(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("md") => "note",
        Some("canvas") => "canvas",
        _ => "other",
    }
}

/// Detects the encoding of a file that already exists.
fn detect_encoding(path: &Path) -> EncodingKind {
    let Ok(bytes) = std::fs::read(path) else {
        return EncodingKind::Utf8;
    };
    let head = &bytes[..bytes.len().min(4096)];
    EncodingKind::detect(head)
}

/// Detects the line ending of a file that already exists.
fn detect_line_ending(path: &Path) -> LineEnding {
    let Ok(bytes) = std::fs::read(path) else {
        return LineEnding::Lf;
    };
    let head = &bytes[..bytes.len().min(4096)];
    let text = String::from_utf8_lossy(head);
    LineEnding::detect(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcpad_core::session::{close_tab, open_tab};

    fn vault_with_notes() -> (tempfile::TempDir, VaultService) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("notes")).unwrap();
        std::fs::write(dir.path().join("notes/a.md"), "# A\n\nпервая\n").unwrap();
        std::fs::write(dir.path().join("notes/b.md"), "# B\n").unwrap();
        let service = VaultService::open(dir.path()).unwrap();
        (dir, service)
    }

    /// A text that is a little larger than the lazy loading threshold.
    fn huge_text() -> String {
        let line = "строка содержимого для большого файла\n";
        let mut text = String::new();
        while (text.len() as u64) <= LARGE_FILE_THRESHOLD {
            text.push_str(line);
        }
        text
    }

    #[test]
    fn lists_the_files_of_the_vault() {
        let (_dir, service) = vault_with_notes();
        let files = service.list_files().unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|file| file.kind == "note"));
        assert!(files.iter().any(|file| file.path == "notes/a.md"));
    }

    #[test]
    fn reads_a_file_with_its_encoding() {
        let (_dir, service) = vault_with_notes();
        let document = service.read_file("notes/a.md").unwrap();
        assert_eq!(document.text, "# A\n\nпервая\n");
        assert_eq!(document.encoding, "UTF-8");
        assert_eq!(document.line_ending, "LF");
        assert!(!document.large);
    }

    #[test]
    fn writes_a_file_atomically_and_keeps_the_encoding() {
        let (_dir, service) = vault_with_notes();
        service
            .write_file(
                "notes/a.md",
                "# A\n\nвторая\n",
                Some("UTF-8"),
                Some("LF"),
                true,
            )
            .unwrap();

        let raw = std::fs::read(service.root().join("notes/a.md")).unwrap();
        assert_eq!(
            raw,
            b"# A\n\n\xd0\xb2\xd1\x82\xd0\xbe\xd1\x80\xd0\xb0\xd1\x8f\n"
        );
    }

    #[test]
    fn writes_a_file_with_crlf_and_a_legacy_encoding() {
        let (_dir, service) = vault_with_notes();
        service
            .write_file(
                "notes/a.md",
                "один\nдва\n",
                Some("Windows-1251"),
                Some("CRLF"),
                true,
            )
            .unwrap();

        let document = service.read_file("notes/a.md").unwrap();
        assert_eq!(document.text, "один\r\nдва\r\n");
        assert_eq!(document.encoding, "Windows-1251");
        assert_eq!(document.line_ending, "CRLF");
    }

    #[test]
    fn converts_a_file_in_place() {
        let (_dir, service) = vault_with_notes();
        service
            .convert_file("notes/a.md", Some("UTF-16 LE"), Some("CRLF"))
            .unwrap();

        let document = service.read_file("notes/a.md").unwrap();
        assert_eq!(document.encoding, "UTF-16 LE");
        assert_eq!(document.line_ending, "CRLF");
        assert!(document.text.contains("первая"));
    }

    #[test]
    fn an_unknown_encoding_is_rejected() {
        let (_dir, service) = vault_with_notes();
        let error = service
            .write_file("notes/a.md", "x", Some("ROT13"), Some("LF"), true)
            .unwrap_err();
        assert!(matches!(error, Error::Config(_)));
    }

    #[test]
    fn a_huge_file_is_opened_partially_and_can_be_loaded_fully() {
        let dir = tempfile::tempdir().unwrap();
        let text = huge_text();
        std::fs::write(dir.path().join("big.md"), &text).unwrap();

        let service = VaultService::open(dir.path()).unwrap();

        let partial = service.read_file("big.md").unwrap();
        assert!(partial.large, "a file above the threshold opens partially");
        assert!(partial.text.len() < text.len());

        let full = service.read_file_full("big.md").unwrap();
        assert!(!full.large);
        assert_eq!(full.text, text);
    }

    #[test]
    fn a_partially_loaded_file_is_never_written_back() {
        let dir = tempfile::tempdir().unwrap();
        let text = huge_text();
        std::fs::write(dir.path().join("big.md"), &text).unwrap();

        let service = VaultService::open(dir.path()).unwrap();

        let error = service
            .write_file("big.md", "кусок\n", Some("UTF-8"), Some("LF"), false)
            .unwrap_err();
        assert!(matches!(error, Error::Config(_)));

        // The file on disk is untouched.
        assert_eq!(
            std::fs::read_to_string(dir.path().join("big.md")).unwrap(),
            text
        );
    }

    #[test]
    fn the_session_survives_a_restart_of_the_service() {
        let dir = tempfile::tempdir().unwrap();
        {
            let service = VaultService::open(dir.path()).unwrap();
            let mut session = Session::default();
            open_tab(&mut session, "notes/a.md");
            service.save_session(&session).unwrap();
        }

        let service = VaultService::open(dir.path()).unwrap();
        let session = service.load_session();
        assert_eq!(session.tabs.len(), 1);
        assert_eq!(session.tabs[0].path, "notes/a.md");
    }

    #[test]
    fn drafts_survive_a_restart_and_can_be_discarded() {
        let dir = tempfile::tempdir().unwrap();
        {
            let service = VaultService::open(dir.path()).unwrap();
            service.save_draft("notes/a.md", "черновик").unwrap();
        }

        let service = VaultService::open(dir.path()).unwrap();
        assert_eq!(
            service.load_draft("notes/a.md").as_deref(),
            Some("черновик")
        );
        service.discard_draft("notes/a.md").unwrap();
        assert!(service.load_draft("notes/a.md").is_none());
    }

    #[test]
    fn the_service_files_never_show_up_in_the_file_list() {
        let (_dir, service) = vault_with_notes();
        service.save_draft("notes/a.md", "черновик").unwrap();
        service.save_session(&Session::default()).unwrap();

        let files = service.list_files().unwrap();
        assert!(files.iter().all(|file| !file.path.contains(".mcpad")));
    }

    #[test]
    fn closing_a_tab_frees_the_active_one() {
        let mut session = Session::default();
        open_tab(&mut session, "notes/a.md");
        open_tab(&mut session, "notes/b.md");
        close_tab(&mut session, "notes/b.md");
        assert_eq!(session.active.as_deref(), Some("notes/a.md"));
    }

    #[test]
    fn a_path_outside_of_the_vault_is_rejected() {
        let (_dir, service) = vault_with_notes();
        let error = service.read_file("../secret.md").unwrap_err();
        assert!(matches!(error, Error::Config(_)));

        let error = service
            .write_file("../secret.md", "x", None, None, true)
            .unwrap_err();
        assert!(matches!(error, Error::Config(_)));

        let error = service.read_file("/etc/passwd").unwrap_err();
        assert!(matches!(error, Error::Config(_)));
    }

    #[test]
    fn the_service_directory_is_never_written() {
        let (_dir, service) = vault_with_notes();
        let error = service
            .write_file(".mcpad/state.json", "{}", None, None, true)
            .unwrap_err();
        assert!(matches!(error, Error::Config(_)));

        let error = service.delete_file(".mcpad/drafts/a.md").unwrap_err();
        assert!(matches!(error, Error::Config(_)));
    }

    #[test]
    fn deletes_a_file_of_the_vault() {
        let (dir, service) = vault_with_notes();
        service.delete_file("notes/a.md").unwrap();
        assert!(!dir.path().join("notes/a.md").exists());

        let error = service.delete_file("notes/a.md").unwrap_err();
        assert!(matches!(error, Error::Io { .. }));
    }

    #[test]
    fn a_directory_is_never_deleted() {
        let (_dir, service) = vault_with_notes();
        let error = service.delete_file("notes").unwrap_err();
        assert!(matches!(error, Error::Config(_)));
    }

    #[test]
    fn searches_the_text_of_the_vault() {
        let (_dir, service) = vault_with_notes();
        let hits = service.search("первая", &SearchOptions::default()).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, "notes/a.md");
        assert_eq!(hits[0].line, 3);
    }
}
