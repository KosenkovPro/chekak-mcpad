//! The vault: a plain directory with plain files.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::fs::walk_files;
use crate::ignore::IgnoreRules;
use crate::paths::{VaultLayout, SERVICE_DIR_NAME};

/// An opened vault.
#[derive(Debug, Clone)]
pub struct Vault {
    layout: VaultLayout,
    ignore: IgnoreRules,
}

impl Vault {
    /// Opens a vault, creating the service directory when needed.
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        let meta = std::fs::metadata(root).map_err(|e| Error::io(root, e))?;
        if !meta.is_dir() {
            return Err(Error::NotADirectory(root.to_path_buf()));
        }

        let layout = VaultLayout::new(root);
        let service_dir = layout.service_dir();
        std::fs::create_dir_all(&service_dir).map_err(|e| Error::io(&service_dir, e))?;
        std::fs::create_dir_all(layout.drafts_dir())
            .map_err(|e| Error::io(layout.drafts_dir(), e))?;

        let ignore = IgnoreRules::load(root)?;

        Ok(Self { layout, ignore })
    }

    /// The layout of this vault.
    pub fn layout(&self) -> &VaultLayout {
        &self.layout
    }

    /// The vault root.
    pub fn root(&self) -> &Path {
        self.layout.root()
    }

    /// The compiled ignore rules.
    pub fn ignore(&self) -> &IgnoreRules {
        &self.ignore
    }

    /// Whether a relative path points into the service directory.
    pub fn is_service_path(&self, relative: &Path) -> bool {
        relative
            .components()
            .next()
            .map(|component| component.as_os_str() == std::ffi::OsStr::new(SERVICE_DIR_NAME))
            .unwrap_or(false)
    }

    /// Whether a relative path is excluded from synchronisation.
    pub fn is_ignored(&self, relative: &Path) -> bool {
        self.is_service_path(relative) || self.ignore.is_ignored(relative)
    }

    /// Every file that takes part in synchronisation.
    pub fn syncable_files(&self) -> Result<Vec<PathBuf>> {
        let service = std::ffi::OsStr::new(SERVICE_DIR_NAME);
        let files = walk_files(self.root(), |path| {
            path.file_name()
                .map(|name| name == service)
                .unwrap_or(false)
        })?;

        Ok(files
            .into_iter()
            .filter(|path| match self.layout.relative(path) {
                Some(relative) => !self.ignore.is_ignored(&relative),
                None => true,
            })
            .collect())
    }

    /// Converts an absolute path into a vault relative path.
    pub fn relative(&self, path: &Path) -> Option<PathBuf> {
        self.layout.relative(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault_with_files() -> (tempfile::TempDir, Vault) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("notes")).unwrap();
        std::fs::create_dir_all(dir.path().join(".mcpad/drafts")).unwrap();
        std::fs::write(dir.path().join("notes/a.md"), b"# a").unwrap();
        std::fs::write(dir.path().join("notes/b.md"), b"# b").unwrap();
        std::fs::write(dir.path().join(".mcpad/state.json"), b"{}").unwrap();
        std::fs::write(dir.path().join(".mcpad/drafts/a.md"), b"draft").unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        (dir, vault)
    }

    #[test]
    fn opening_creates_the_service_directory() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        assert!(vault.layout().service_dir().is_dir());
    }

    #[test]
    fn service_files_never_take_part_in_sync() {
        let (_dir, vault) = vault_with_files();
        let files = vault.syncable_files().unwrap();
        assert_eq!(files.len(), 2);
        assert!(files
            .iter()
            .all(|p| !p.to_string_lossy().contains(".mcpad")));
    }

    #[test]
    fn ignore_rules_apply_to_syncable_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), b"a").unwrap();
        std::fs::write(dir.path().join("b.mp4"), b"b").unwrap();
        std::fs::write(dir.path().join(".mcpadignore"), "*.mp4\n").unwrap();

        let vault = Vault::open(dir.path()).unwrap();
        let files = vault.syncable_files().unwrap();

        // The excluded file is gone, the note stays and the rule file itself is
        // portable, exactly like `.gitignore` in a Git repository.
        assert!(files.iter().all(|path| !path.ends_with("b.mp4")));
        assert!(files.iter().any(|path| path.ends_with("a.md")));
        assert!(files.iter().any(|path| path.ends_with(".mcpadignore")));
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn non_directory_root_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file.md");
        std::fs::write(&file, b"x").unwrap();
        assert!(matches!(Vault::open(&file), Err(Error::NotADirectory(_))));
    }
}
