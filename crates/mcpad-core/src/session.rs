//! Persistent editor session.
//!
//! The session remembers which files were open, where the caret was and what
//! the unsaved drafts contained. It lives inside the service directory of the
//! vault, so it is never synchronised and never leaves the machine.
//!
//! Drafts are kept in separate files: a draft can be large, and the state file
//! must stay small and cheap to write.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::fs::{atomic_write, ensure_dir};
use crate::paths::VaultLayout;

/// One open tab.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TabState {
    /// Path of the file, relative to the vault root.
    pub path: String,
    /// Caret position as line and column.
    pub caret: Option<(u64, u64)>,
    /// Vertical scroll offset.
    pub scroll: f64,
    /// Whether the tab is pinned.
    pub pinned: bool,
}

/// The persisted session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    /// Open tabs, in the order they were opened.
    pub tabs: Vec<TabState>,
    /// Path of the active tab.
    pub active: Option<String>,
    /// Splits of the editor layout.
    pub splits: Vec<String>,
}

/// An unsaved draft on disk.
///
/// The original path is stored inside the file, because the file name itself is
/// flattened and therefore not reversible.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Draft {
    path: String,
    content: String,
}

/// The session of one vault, stored in its service directory.
#[derive(Debug, Clone)]
pub struct SessionStore {
    layout: VaultLayout,
}

impl SessionStore {
    /// Builds a store for a vault.
    pub fn new(layout: VaultLayout) -> Self {
        Self { layout }
    }

    /// The file the session is stored in.
    pub fn state_file(&self) -> PathBuf {
        self.layout.state_file()
    }

    /// The directory that holds the unsaved drafts.
    pub fn drafts_dir(&self) -> PathBuf {
        self.layout.drafts_dir()
    }

    /// Loads the session. A missing or broken file yields an empty session.
    pub fn load(&self) -> Session {
        let path = self.state_file();
        let Ok(raw) = std::fs::read(&path) else {
            return Session::default();
        };
        serde_json::from_slice(&raw).unwrap_or_default()
    }

    /// Writes the session.
    pub fn save(&self, session: &Session) -> Result<()> {
        let json = serde_json::to_vec_pretty(session)?;
        atomic_write(&self.state_file(), &json)
    }

    /// The file that holds the draft of a path.
    pub fn draft_file(&self, relative: &str) -> PathBuf {
        self.drafts_dir()
            .join(format!("{}.draft", sanitize(relative)))
    }

    /// Stores the unsaved content of a file.
    pub fn save_draft(&self, relative: &str, content: &str) -> Result<()> {
        let dir = self.drafts_dir();
        ensure_dir(&dir)?;
        let draft = Draft {
            path: relative.to_string(),
            content: content.to_string(),
        };
        let json = serde_json::to_vec_pretty(&draft)?;
        atomic_write(&self.draft_file(relative), &json)
    }

    /// Reads the unsaved content of a file.
    pub fn load_draft(&self, relative: &str) -> Option<String> {
        let raw = std::fs::read(self.draft_file(relative)).ok()?;
        let draft: Draft = serde_json::from_slice(&raw).ok()?;
        Some(draft.content)
    }

    /// Removes the unsaved content of a file.
    pub fn discard_draft(&self, relative: &str) -> Result<()> {
        let path = self.draft_file(relative);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(Error::io(path, error)),
        }
    }

    /// Lists the drafts that are waiting to be restored.
    pub fn pending_drafts(&self) -> Vec<String> {
        let dir = self.drafts_dir();
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Vec::new();
        };

        let mut drafts: Vec<String> = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|ext| ext == "draft")
                    .unwrap_or(false)
            })
            .filter_map(|entry| {
                let raw = std::fs::read(entry.path()).ok()?;
                let draft: Draft = serde_json::from_slice(&raw).ok()?;
                Some(draft.path)
            })
            .collect();

        drafts.sort();
        drafts
    }
}

/// Turns a vault relative path into a flat, safe file name.
fn sanitize(relative: &str) -> String {
    relative
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            other => other,
        })
        .collect()
}

/// Adds a tab to the session, keeping the order stable.
pub fn open_tab(session: &mut Session, relative: &str) {
    if !session.tabs.iter().any(|tab| tab.path == relative) {
        session.tabs.push(TabState {
            path: relative.to_string(),
            ..TabState::default()
        });
    }
    session.active = Some(relative.to_string());
}

/// Removes a tab from the session.
pub fn close_tab(session: &mut Session, relative: &str) {
    session.tabs.retain(|tab| tab.path != relative);
    if session.active.as_deref() == Some(relative) {
        session.active = session.tabs.last().map(|tab| tab.path.clone());
    }
}

/// Finds a tab.
pub fn find_tab<'a>(session: &'a Session, relative: &str) -> Option<&'a TabState> {
    session.tabs.iter().find(|tab| tab.path == relative)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, SessionStore) {
        let dir = tempfile::tempdir().unwrap();
        let layout = VaultLayout::new(dir.path());
        (dir, SessionStore::new(layout))
    }

    #[test]
    fn a_missing_session_file_yields_an_empty_session() {
        let (_dir, store) = store();
        let session = store.load();
        assert!(session.tabs.is_empty());
        assert!(session.active.is_none());
    }

    #[test]
    fn the_session_survives_a_restart() {
        let (_dir, store) = store();
        let mut session = Session::default();
        open_tab(&mut session, "notes/a.md");
        open_tab(&mut session, "notes/b.md");
        session.tabs[0].caret = Some((12, 4));
        session.active = Some("notes/a.md".to_string());
        store.save(&session).unwrap();

        let restored = store.load();
        assert_eq!(restored.tabs.len(), 2);
        assert_eq!(restored.active.as_deref(), Some("notes/a.md"));
        assert_eq!(restored.tabs[0].caret, Some((12, 4)));
    }

    #[test]
    fn a_broken_session_file_does_not_break_the_editor() {
        let (dir, store) = store();
        ensure_dir(&store.drafts_dir()).unwrap();
        std::fs::write(store.state_file(), b"{ not json").unwrap();
        assert!(store.load().tabs.is_empty());
        drop(dir);
    }

    #[test]
    fn tabs_are_opened_once_and_closed_by_path() {
        let mut session = Session::default();
        open_tab(&mut session, "a.md");
        open_tab(&mut session, "a.md");
        assert_eq!(session.tabs.len(), 1);

        open_tab(&mut session, "b.md");
        assert_eq!(session.active.as_deref(), Some("b.md"));

        close_tab(&mut session, "b.md");
        assert_eq!(session.tabs.len(), 1);
        assert_eq!(session.active.as_deref(), Some("a.md"));
    }

    #[test]
    fn drafts_round_trip_and_can_be_discarded() {
        let (_dir, store) = store();
        store.save_draft("notes/a.md", "черновик").unwrap();
        assert_eq!(store.load_draft("notes/a.md").as_deref(), Some("черновик"));
        assert_eq!(store.pending_drafts(), vec!["notes/a.md".to_string()]);

        store.discard_draft("notes/a.md").unwrap();
        assert!(store.load_draft("notes/a.md").is_none());
        assert!(store.pending_drafts().is_empty());
    }

    #[test]
    fn draft_file_names_are_flat_and_safe() {
        let (_dir, store) = store();
        let file = store.draft_file("notes/sub dir/a.md");
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        assert!(!name.contains('/'), "{name}");
        assert!(!name.contains('\\'), "{name}");
        assert_eq!(name, "notes_sub dir_a.md.draft");
    }

    #[test]
    fn discarding_a_missing_draft_is_not_an_error() {
        let (_dir, store) = store();
        store.discard_draft("nothing.md").unwrap();
    }
}
