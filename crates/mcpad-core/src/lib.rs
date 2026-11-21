//! MCPad core domain model.
//!
//! Everything that the rest of the product shares lives here: the on-disk vault
//! layout, platform directories, application configuration, `.mcpadignore` rules
//! and atomic file writes.
//!
//! Design rules that the rest of the codebase relies on:
//!
//! * A vault is a plain directory with plain `.md` / `.canvas` files. Nothing in
//!   this crate ever rewrites user content into a proprietary format.
//! * The `.mcpad/` directory holds service state only. It is never synchronised
//!   and never shown in the file tree.
//! * Writes go through [`fs::atomic_write`], so a crash can never leave a half
//!   written note behind.

pub mod config;
pub mod error;
pub mod fs;
pub mod ignore;
pub mod paths;
pub mod session;
pub mod theme;
pub mod vault;

pub use config::{AppConfig, AppLockConfig, EditorConfig, SyncConfig, ThemeConfig};
pub use error::{Error, Result};
pub use ignore::IgnoreRules;
pub use paths::{AppPaths, VaultLayout};
pub use session::{Session, SessionStore, TabState};
pub use theme::{Theme, ThemeMode};
pub use vault::Vault;
