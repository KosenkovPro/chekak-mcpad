//! `.mcpadignore` rules.
//!
//! The file lives in the vault root and works like `.gitignore`: one glob per
//! line, `#` starts a comment, empty lines are ignored. Patterns are matched
//! against the vault relative path and against every path component, so
//! `*.mp4` also excludes `media/clip.mp4`.

use std::path::Path;

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

use crate::error::{Error, Result};

/// Compiled ignore rules.
#[derive(Debug, Clone, Default)]
pub struct IgnoreRules {
    patterns: Vec<String>,
    globs: GlobSet,
}

impl IgnoreRules {
    /// Rules without any pattern.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Compiles the given patterns.
    pub fn from_patterns<I, S>(patterns: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut builder = GlobSetBuilder::new();
        let mut kept = Vec::new();

        for pattern in patterns {
            let pattern = pattern.as_ref().trim();
            if pattern.is_empty() || pattern.starts_with('#') {
                continue;
            }
            let glob = GlobBuilder::new(pattern)
                .literal_separator(true)
                .build()
                .map_err(|e| Error::Glob {
                    pattern: pattern.to_string(),
                    message: e.to_string(),
                })?;
            builder.add(glob);
            kept.push(pattern.to_string());
        }

        let globs = builder.build().map_err(|e| Error::Glob {
            pattern: kept.join(", "),
            message: e.to_string(),
        })?;

        Ok(Self {
            patterns: kept,
            globs,
        })
    }

    /// Loads `.mcpadignore` from a vault root. A missing file means no rules.
    pub fn load(vault_root: &Path) -> Result<Self> {
        let path = vault_root.join(".mcpadignore");
        match std::fs::read_to_string(&path) {
            Ok(content) => Self::from_patterns(content.lines()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::empty()),
            Err(e) => Err(Error::io(path, e)),
        }
    }

    /// The raw patterns, in file order.
    pub fn patterns(&self) -> &[String] {
        &self.patterns
    }

    /// Whether a vault relative path is excluded.
    pub fn is_ignored(&self, relative: &Path) -> bool {
        if self.patterns.is_empty() {
            return false;
        }
        if self.globs.is_match(relative) {
            return true;
        }
        relative
            .components()
            .any(|component| self.globs.is_match(Path::new(component.as_os_str())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_rules_ignore_nothing() {
        let rules = IgnoreRules::empty();
        assert!(!rules.is_ignored(Path::new("notes/a.md")));
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let rules = IgnoreRules::from_patterns(["# comment", "", "   ", "*.mp4"]).unwrap();
        assert_eq!(rules.patterns().len(), 1);
    }

    #[test]
    fn extension_patterns_match_any_depth() {
        let rules = IgnoreRules::from_patterns(["*.mp4"]).unwrap();
        assert!(rules.is_ignored(Path::new("clip.mp4")));
        assert!(rules.is_ignored(Path::new("media/clip.mp4")));
        assert!(!rules.is_ignored(Path::new("media/clip.mkv")));
    }

    #[test]
    fn directory_patterns_use_double_star() {
        let rules = IgnoreRules::from_patterns(["**/build/**"]).unwrap();
        assert!(rules.is_ignored(Path::new("build/out.txt")));
        assert!(rules.is_ignored(Path::new("src/build/out.txt")));
        assert!(!rules.is_ignored(Path::new("src/main.rs")));
    }

    #[test]
    fn loads_from_disk_and_missing_file_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let rules = IgnoreRules::load(dir.path()).unwrap();
        assert!(rules.patterns().is_empty());

        std::fs::write(dir.path().join(".mcpadignore"), "*.iso\n").unwrap();
        let rules = IgnoreRules::load(dir.path()).unwrap();
        assert!(rules.is_ignored(Path::new("disk.iso")));
    }
}
