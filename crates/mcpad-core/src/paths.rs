//! Platform directories and the on-disk layout of a vault.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Directory name used for all MCPad state inside the platform directories.
pub const APP_DIR_NAME: &str = "MCPad";

/// Service directory inside a vault. Never synchronised, never shown in the tree.
pub const SERVICE_DIR_NAME: &str = ".mcpad";

/// Platform specific locations for application level data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    /// User configuration (theme selection, locale, editor preferences).
    pub config_dir: PathBuf,
    /// Rebuildable and portable state (indices, downloaded models, drafts cache).
    pub data_dir: PathBuf,
    /// Disposable caches (Tantivy index, thumbnail cache).
    pub cache_dir: PathBuf,
    /// Rotating application logs.
    pub log_dir: PathBuf,
}

impl AppPaths {
    /// Detects the platform directories.
    ///
    /// * Windows: `%APPDATA%\MCPad` (config), `%LOCALAPPDATA%\MCPad` (data)
    /// * macOS: `~/Library/Application Support/MCPad`
    /// * Linux: `~/.config/MCPad` and `~/.local/share/MCPad`
    pub fn detect() -> Result<Self> {
        let config_root = dirs::config_dir().ok_or(Error::NoHomeDir)?;
        let data_root = dirs::data_local_dir()
            .or_else(dirs::data_dir)
            .ok_or(Error::NoHomeDir)?;
        let cache_root = dirs::cache_dir().unwrap_or_else(|| data_root.join("cache"));

        let config_dir = config_root.join(APP_DIR_NAME);
        let data_dir = data_root.join(APP_DIR_NAME);
        let cache_dir = cache_root.join(APP_DIR_NAME);
        let log_dir = data_dir.join("logs");

        Ok(Self {
            config_dir,
            data_dir,
            cache_dir,
            log_dir,
        })
    }

    /// `config.json` inside the configuration directory.
    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.json")
    }

    /// Root of the Tantivy indexes, one directory per vault.
    pub fn index_dir(&self) -> PathBuf {
        self.cache_dir.join("index")
    }

    /// Directory holding the downloaded Whisper models.
    pub fn models_dir(&self) -> PathBuf {
        self.data_dir.join("models")
    }

    /// Creates every directory the application expects to exist.
    pub fn ensure(&self) -> Result<()> {
        for dir in [
            &self.config_dir,
            &self.data_dir,
            &self.cache_dir,
            &self.log_dir,
            &self.index_dir(),
            &self.models_dir(),
        ] {
            std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
        }
        Ok(())
    }
}

/// The on-disk layout of a single vault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultLayout {
    root: PathBuf,
}

impl VaultLayout {
    /// Builds a layout for the given root directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The vault root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Service directory, local only.
    pub fn service_dir(&self) -> PathBuf {
        self.root.join(SERVICE_DIR_NAME)
    }

    /// Persisted editor state (open tabs, caret positions, panel sizes).
    pub fn state_file(&self) -> PathBuf {
        self.service_dir().join("state.json")
    }

    /// Hot-exit drafts.
    pub fn drafts_dir(&self) -> PathBuf {
        self.service_dir().join("drafts")
    }

    /// Window sessions.
    pub fn sessions_dir(&self) -> PathBuf {
        self.service_dir().join("sessions")
    }

    /// Conventional folder for binary attachments (photos imported from Kipass, etc.).
    pub fn attachments_dir(&self) -> PathBuf {
        self.root.join("attachments")
    }

    /// The `.mcpadignore` rule file.
    pub fn ignore_file(&self) -> PathBuf {
        self.root.join(".mcpadignore")
    }

    /// Converts an absolute path into a vault relative path.
    pub fn relative(&self, path: &Path) -> Option<PathBuf> {
        path.strip_prefix(&self.root).ok().map(Path::to_path_buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_points_inside_the_root() {
        let layout = VaultLayout::new("/vault");
        assert!(layout.service_dir().starts_with("/vault"));
        assert!(layout.state_file().ends_with("state.json"));
        assert!(layout.attachments_dir().ends_with("attachments"));
    }

    #[test]
    fn relative_strips_the_root() {
        let layout = VaultLayout::new("/vault");
        assert_eq!(
            layout.relative(Path::new("/vault/notes/a.md")),
            Some(PathBuf::from("notes/a.md"))
        );
        assert_eq!(layout.relative(Path::new("/other/a.md")), None);
    }
}
