//! The vault that was open last.
//!
//! The application remembers the vault of the previous run in the platform
//! configuration, so the next start reopens it without a dialog. The value is
//! only a hint: a vault that disappeared is simply ignored.

use std::path::{Path, PathBuf};

use mcpad_core::config::AppConfig;
use mcpad_core::paths::AppPaths;
use mcpad_core::Result;

/// Reads the vault that was open last from the given configuration file.
pub fn last_vault_in(config_file: &Path) -> Option<PathBuf> {
    let config = AppConfig::load(config_file).unwrap_or_default();
    config.last_vault.filter(|root| root.is_dir())
}

/// Writes the vault that was open last into the given configuration file.
pub fn remember_vault_in(config_file: &Path, root: &Path) -> Result<()> {
    let mut config = AppConfig::load(config_file).unwrap_or_default();
    config.last_vault = Some(root.to_path_buf());
    config.save(config_file)
}

/// Reads the vault that was open last from the platform configuration.
pub fn last_vault() -> Option<PathBuf> {
    let paths = AppPaths::detect().ok()?;
    last_vault_in(&paths.config_file())
}

/// Writes the vault that was open last into the platform configuration.
pub fn remember_vault(root: &Path) -> Result<()> {
    let paths = AppPaths::detect()?;
    paths.ensure()?;
    remember_vault_in(&paths.config_file(), root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vault_is_remembered_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault");
        std::fs::create_dir_all(&vault).unwrap();
        let config = dir.path().join("config.json");

        remember_vault_in(&config, &vault).unwrap();
        assert_eq!(last_vault_in(&config), Some(vault));
    }

    #[test]
    fn a_vault_that_disappeared_is_forgotten() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config.json");
        remember_vault_in(&config, &dir.path().join("gone")).unwrap();

        assert_eq!(last_vault_in(&config), None);
    }

    #[test]
    fn a_broken_configuration_does_not_break_the_start() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config.json");
        std::fs::write(&config, b"not json at all").unwrap();

        assert_eq!(last_vault_in(&config), None);
    }
}
