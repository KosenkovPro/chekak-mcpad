//! Application configuration.
//!
//! The configuration is intentionally boring: a single JSON document that is
//! validated with serde, stored in the platform configuration directory and
//! written atomically. Every field has a default, so an empty or partially
//! written file never breaks the application.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::fs::atomic_write;

/// Root configuration document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Lock screen / secrets protection. Disabled by default so the editor
    /// opens instantly, exactly like Notepad++ does.
    pub app_lock: AppLockConfig,
    /// Editor behaviour.
    pub editor: EditorConfig,
    /// Synchronisation behaviour.
    pub sync: SyncConfig,
    /// Theme selection.
    pub theme: ThemeConfig,
    /// Interface language, `ru` or `en`.
    pub locale: String,
    /// The vault that was open last, reopened on the next start.
    pub last_vault: Option<std::path::PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_lock: AppLockConfig::default(),
            editor: EditorConfig::default(),
            sync: SyncConfig::default(),
            theme: ThemeConfig::default(),
            locale: "ru".to_string(),
            last_vault: None,
        }
    }
}

/// Optional application lock.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppLockConfig {
    /// Master switch. Off by default.
    pub enabled: bool,
    /// Hash of the lock secret, kept in the OS keyring together with the
    /// application secrets, never in this file.
    pub pin_hash: Option<String>,
    /// Ask for the secret on every start.
    pub lock_on_start: bool,
    /// Automatic lock after N minutes of inactivity, `0` disables it.
    pub auto_lock_minutes: u32,
}

impl Default for AppLockConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            pin_hash: None,
            lock_on_start: true,
            auto_lock_minutes: 0,
        }
    }
}

/// Editor behaviour.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    /// Preferred encoding label, see `mcpad_text::EncodingKind`.
    pub encoding: String,
    /// Preferred line ending, `lf` or `crlf`.
    pub line_ending: String,
    /// Tab width in spaces.
    pub tab_width: usize,
    /// Autosave debounce in milliseconds.
    pub autosave_ms: u64,
    /// Files above this size are opened in lazy mode instead of a full rope.
    pub large_file_threshold_bytes: u64,
    /// Remember the cursor position of every note.
    pub restore_caret: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            encoding: "UTF-8".to_string(),
            line_ending: "lf".to_string(),
            tab_width: 4,
            autosave_ms: 1500,
            large_file_threshold_bytes: 8 * 1024 * 1024,
            restore_caret: true,
        }
    }
}

/// Synchronisation behaviour.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncConfig {
    /// Master switch for background synchronisation.
    pub enabled: bool,
    /// Active provider: `git`, `s3`, `webdav` or `cloud`.
    pub provider: Option<String>,
    /// Files larger than this are not synchronised.
    pub max_file_size_mb: u64,
    /// End to end encryption of the synchronised payloads.
    pub e2e_enabled: bool,
    /// Extra exclusions on top of `.mcpadignore`.
    pub exclude_patterns: Vec<String>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: None,
            max_file_size_mb: 25,
            e2e_enabled: false,
            exclude_patterns: Vec::new(),
        }
    }
}

/// Theme selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    /// `dark`, `light` or `high-contrast`.
    pub mode: String,
    /// `compact`, `default` or `comfortable`.
    pub density: String,
    /// Path to a user provided theme file.
    pub custom_theme: Option<std::path::PathBuf>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            mode: "dark".to_string(),
            density: "default".to_string(),
            custom_theme: None,
        }
    }
}

impl AppConfig {
    /// Loads the configuration, falling back to defaults when the file is absent.
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(Error::from),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(Error::io(path, e)),
        }
    }

    /// Writes the configuration atomically.
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_vec_pretty(self)?;
        atomic_write(path, &json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_agreed_product_decisions() {
        let cfg = AppConfig::default();
        assert!(!cfg.app_lock.enabled, "app lock must be opt-in");
        assert_eq!(cfg.sync.max_file_size_mb, 25);
        assert!(!cfg.sync.e2e_enabled);
        assert_eq!(cfg.locale, "ru");
    }

    #[test]
    fn missing_file_yields_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = AppConfig::load(&dir.path().join("config.json")).unwrap();
        assert_eq!(cfg.editor.tab_width, 4);
    }

    #[test]
    fn round_trip_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut cfg = AppConfig::default();
        cfg.theme.mode = "light".to_string();
        cfg.save(&path).unwrap();

        let loaded = AppConfig::load(&path).unwrap();
        assert_eq!(loaded.theme.mode, "light");
    }

    #[test]
    fn partial_documents_fall_back_to_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, br#"{"theme":{"mode":"light"}}"#).unwrap();
        let loaded = AppConfig::load(&path).unwrap();
        assert_eq!(loaded.theme.mode, "light");
        assert_eq!(loaded.editor.tab_width, 4);
        assert!(loaded.last_vault.is_none());
    }

    #[test]
    fn the_last_vault_survives_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let cfg = AppConfig {
            last_vault: Some(std::path::PathBuf::from("/vault")),
            ..AppConfig::default()
        };
        cfg.save(&path).unwrap();

        let loaded = AppConfig::load(&path).unwrap();
        assert_eq!(
            loaded.last_vault.as_deref(),
            Some(std::path::Path::new("/vault"))
        );
    }
}
