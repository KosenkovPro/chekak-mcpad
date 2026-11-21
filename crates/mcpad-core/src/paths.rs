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
        let dirs = PlatformDirs::current();

        let config_root = dirs.config.ok_or(Error::NoHomeDir)?;
        let data_root = dirs.data.ok_or(Error::NoHomeDir)?;
        let cache_root = dirs.cache.unwrap_or_else(|| data_root.join("cache"));

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

/// The directories of the platform, as the operating system defines them.
///
/// The rules are implemented here instead of taken from a crate, because the
/// only crate that offers them brings a copyleft licence into the core.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct PlatformDirs {
    /// Where the user keeps the configuration.
    config: Option<PathBuf>,
    /// Where the user keeps the data of the application.
    data: Option<PathBuf>,
    /// Where the user keeps the disposable caches.
    cache: Option<PathBuf>,
}

impl PlatformDirs {
    /// Reads the directories of the current platform from the environment.
    fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::windows(
                env_path("USERPROFILE").as_deref(),
                env_path("APPDATA").as_deref(),
                env_path("LOCALAPPDATA").as_deref(),
            )
        } else if cfg!(target_os = "macos") {
            Self::macos(env_path("HOME").as_deref())
        } else {
            Self::unix(
                env_path("HOME").as_deref(),
                env_path("XDG_CONFIG_HOME").as_deref(),
                env_path("XDG_DATA_HOME").as_deref(),
                env_path("XDG_CACHE_HOME").as_deref(),
            )
        }
    }

    /// Windows: the roaming `%APPDATA%` and the local `%LOCALAPPDATA%`.
    fn windows(home: Option<&Path>, appdata: Option<&Path>, local_appdata: Option<&Path>) -> Self {
        let roaming = appdata
            .map(Path::to_path_buf)
            .or_else(|| home.map(|home| home.join("AppData").join("Roaming")));
        let local = local_appdata
            .map(Path::to_path_buf)
            .or_else(|| home.map(|home| home.join("AppData").join("Local")));

        Self {
            config: roaming.clone(),
            data: local.clone(),
            cache: local,
        }
    }

    /// macOS: the application support folder and the caches folder.
    fn macos(home: Option<&Path>) -> Self {
        let support = home.map(|home| home.join("Library").join("Application Support"));

        Self {
            config: support.clone(),
            data: support,
            cache: home.map(|home| home.join("Library").join("Caches")),
        }
    }

    /// Linux and the other systems of that family: the XDG directories.
    fn unix(
        home: Option<&Path>,
        xdg_config: Option<&Path>,
        xdg_data: Option<&Path>,
        xdg_cache: Option<&Path>,
    ) -> Self {
        let config = absolute(xdg_config).or_else(|| home.map(|home| home.join(".config")));
        let data =
            absolute(xdg_data).or_else(|| home.map(|home| home.join(".local").join("share")));
        let cache = absolute(xdg_cache).or_else(|| home.map(|home| home.join(".cache")));

        Self {
            config,
            data: data.clone(),
            cache,
        }
    }
}

/// Reads a path from the environment.
fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

/// Keeps a path only when it is absolute, as the XDG rules ask.
fn absolute(path: Option<&Path>) -> Option<PathBuf> {
    path.filter(|path| path.is_absolute())
        .map(Path::to_path_buf)
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

    /// An absolute directory of the current platform.
    fn absolute_dir(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("the working directory")
            .join(name)
    }

    #[test]
    fn windows_directories_follow_the_environment() {
        let roaming = absolute_dir("Roaming");
        let local = absolute_dir("Local");
        let dirs = PlatformDirs::windows(Some(&absolute_dir("home")), Some(&roaming), Some(&local));

        assert_eq!(dirs.config, Some(roaming.clone()));
        assert_eq!(dirs.data, Some(local.clone()));
        assert_eq!(dirs.cache, Some(local));
    }

    #[test]
    fn windows_directories_fall_back_to_the_home_directory() {
        let home = absolute_dir("home");
        let dirs = PlatformDirs::windows(Some(&home), None, None);

        assert_eq!(dirs.config, Some(home.join("AppData").join("Roaming")));
        assert_eq!(dirs.data, Some(home.join("AppData").join("Local")));
    }

    #[test]
    fn macos_directories_live_in_the_library() {
        let home = absolute_dir("Users/user");
        let dirs = PlatformDirs::macos(Some(&home));

        assert_eq!(
            dirs.config,
            Some(home.join("Library").join("Application Support"))
        );
        assert_eq!(
            dirs.data,
            Some(home.join("Library").join("Application Support"))
        );
        assert_eq!(dirs.cache, Some(home.join("Library").join("Caches")));
    }

    #[test]
    fn unix_directories_follow_the_xdg_rules() {
        let home = absolute_dir("home/user");
        let config = absolute_dir("etc/xdg");
        let dirs = PlatformDirs::unix(Some(&home), Some(&config), None, None);

        assert_eq!(dirs.config, Some(config));
        assert_eq!(dirs.data, Some(home.join(".local").join("share")));
        assert_eq!(dirs.cache, Some(home.join(".cache")));
    }

    #[test]
    fn relative_xdg_paths_are_ignored() {
        let home = absolute_dir("home/user");
        let dirs = PlatformDirs::unix(Some(&home), Some(Path::new("relative/config")), None, None);

        assert_eq!(dirs.config, Some(home.join(".config")));
    }

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
