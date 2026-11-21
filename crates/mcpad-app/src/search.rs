//! Full text search across the files of a vault.
//!
//! The search is line based and works on decoded text, so a note written in
//! Windows-1251 or UTF-16 is searched just like a UTF-8 note. It is the same
//! engine the MCP server exposes to AI agents, therefore it stays small,
//! predictable and free of any index that could go stale.

use std::path::{Path, PathBuf};

use mcpad_core::{Error, Result};
use mcpad_text::Document;
use regex::{Regex, RegexBuilder};
use serde::Serialize;

/// The longest fragment of a matching line that is reported to the caller.
pub const HIT_TEXT_LIMIT: usize = 400;

/// How a search is performed.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Treat the query as a regular expression instead of plain text.
    pub regex: bool,
    /// Compare the query with the text exactly, ignoring the letter case.
    pub case_sensitive: bool,
    /// Only look inside this directory of the vault, for example `notes`.
    pub prefix: Option<String>,
    /// Stop the search after this many matches.
    pub max_results: usize,
    /// Skip files that are bigger than this.
    pub max_file_bytes: u64,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            regex: false,
            case_sensitive: false,
            prefix: None,
            max_results: 200,
            max_file_bytes: 4 * 1024 * 1024,
        }
    }
}

/// A single match inside a file.
#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    /// Path of the file relative to the vault root.
    pub path: String,
    /// Number of the line, counted from one.
    pub line: u64,
    /// The text of the line, shortened when it is very long.
    pub text: String,
}

/// Searches the given files of a vault.
///
/// The files are absolute, the hits carry a path relative to `root`.
pub fn search_files(
    root: &Path,
    files: &[PathBuf],
    query: &str,
    options: &SearchOptions,
) -> Result<Vec<SearchHit>> {
    let matcher = Matcher::new(query, options)?;
    let mut hits = Vec::new();

    for absolute in files {
        if hits.len() >= options.max_results {
            break;
        }

        let relative = relative_of(root, absolute);
        if !matches_prefix(&relative, options.prefix.as_deref()) {
            continue;
        }

        let Ok(metadata) = std::fs::metadata(absolute) else {
            continue;
        };
        if metadata.len() > options.max_file_bytes {
            continue;
        }

        let Ok(bytes) = std::fs::read(absolute) else {
            continue;
        };
        let document = Document::from_bytes(&bytes);

        for (index, line) in document.text().lines().enumerate() {
            if hits.len() >= options.max_results {
                break;
            }
            if matcher.matches(line) {
                hits.push(SearchHit {
                    path: relative.clone(),
                    line: index as u64 + 1,
                    text: shorten(line),
                });
            }
        }
    }

    Ok(hits)
}

/// The compiled query.
enum Matcher {
    /// A regular expression.
    Pattern(Regex),
    /// A plain text fragment.
    Literal {
        needle: String,
        case_sensitive: bool,
    },
}

impl Matcher {
    fn new(query: &str, options: &SearchOptions) -> Result<Self> {
        if options.regex {
            let pattern = RegexBuilder::new(query)
                .case_insensitive(!options.case_sensitive)
                .size_limit(16 * 1024 * 1024)
                .build()
                .map_err(|error| Error::Config(format!("invalid regular expression: {error}")))?;
            return Ok(Matcher::Pattern(pattern));
        }

        Ok(Matcher::Literal {
            needle: if options.case_sensitive {
                query.to_string()
            } else {
                query.to_lowercase()
            },
            case_sensitive: options.case_sensitive,
        })
    }

    fn matches(&self, line: &str) -> bool {
        match self {
            Matcher::Pattern(pattern) => pattern.is_match(line),
            Matcher::Literal {
                needle,
                case_sensitive,
            } => {
                if *case_sensitive {
                    line.contains(needle.as_str())
                } else {
                    line.to_lowercase().contains(needle.as_str())
                }
            }
        }
    }
}

/// Whether a relative path lies inside the requested prefix.
fn matches_prefix(relative: &str, prefix: Option<&str>) -> bool {
    let Some(prefix) = prefix else {
        return true;
    };
    let prefix = prefix.trim_matches('/');
    if prefix.is_empty() {
        return true;
    }
    relative == prefix || relative.starts_with(&format!("{prefix}/"))
}

/// Turns an absolute path into a vault relative path with forward slashes.
fn relative_of(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Shortens a long line so the answer stays readable.
fn shorten(line: &str) -> String {
    let trimmed = line.trim_end();
    if trimmed.chars().count() <= HIT_TEXT_LIMIT {
        return trimmed.to_string();
    }
    let head: String = trimmed.chars().take(HIT_TEXT_LIMIT).collect();
    format!("{head}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault_with_files() -> (tempfile::TempDir, Vec<PathBuf>) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("notes")).unwrap();
        std::fs::write(dir.path().join("notes/a.md"), "# A\nпервая строка\n").unwrap();
        std::fs::write(dir.path().join("notes/b.md"), "# B\nдругая строка\n").unwrap();
        std::fs::write(dir.path().join("top.md"), "верхний файл\n").unwrap();

        let files = vec![
            dir.path().join("notes/a.md"),
            dir.path().join("notes/b.md"),
            dir.path().join("top.md"),
        ];
        (dir, files)
    }

    #[test]
    fn finds_a_plain_fragment_in_every_file() {
        let (dir, files) = vault_with_files();
        let options = SearchOptions {
            case_sensitive: true,
            ..SearchOptions::default()
        };

        let hits = search_files(dir.path(), &files, "строка", &options).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].path, "notes/a.md");
        assert_eq!(hits[0].line, 2);
        assert_eq!(hits[0].text, "первая строка");
    }

    #[test]
    fn ignores_the_letter_case_by_default() {
        let (dir, files) = vault_with_files();
        let hits = search_files(dir.path(), &files, "СТРОКА", &SearchOptions::default()).unwrap();
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn a_regular_expression_is_supported() {
        let (dir, files) = vault_with_files();
        let options = SearchOptions {
            regex: true,
            case_sensitive: true,
            ..SearchOptions::default()
        };

        let hits = search_files(dir.path(), &files, "^# [AB]$", &options).unwrap();
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn a_broken_regular_expression_is_reported() {
        let (dir, files) = vault_with_files();
        let options = SearchOptions {
            regex: true,
            ..SearchOptions::default()
        };

        let error = search_files(dir.path(), &files, "([", &options).unwrap_err();
        assert!(matches!(error, Error::Config(_)));
    }

    #[test]
    fn the_search_can_be_limited_to_a_directory() {
        let (dir, files) = vault_with_files();
        let options = SearchOptions {
            prefix: Some("notes".to_string()),
            case_sensitive: true,
            ..SearchOptions::default()
        };

        let hits = search_files(dir.path(), &files, "строка", &options).unwrap();
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|hit| hit.path.starts_with("notes/")));
    }

    #[test]
    fn the_number_of_matches_is_limited() {
        let (dir, files) = vault_with_files();
        let options = SearchOptions {
            max_results: 1,
            case_sensitive: true,
            ..SearchOptions::default()
        };

        let hits = search_files(dir.path(), &files, "строка", &options).unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn a_file_that_is_too_big_is_skipped() {
        let (dir, files) = vault_with_files();
        let options = SearchOptions {
            max_file_bytes: 1,
            case_sensitive: true,
            ..SearchOptions::default()
        };

        let hits = search_files(dir.path(), &files, "строка", &options).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn a_long_line_is_shortened() {
        let dir = tempfile::tempdir().unwrap();
        let long = "я".repeat(HIT_TEXT_LIMIT + 50);
        std::fs::write(dir.path().join("long.md"), &long).unwrap();
        let files = vec![dir.path().join("long.md")];

        let hits = search_files(dir.path(), &files, "я", &SearchOptions::default()).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].text.ends_with('…'));
        assert_eq!(hits[0].text.chars().count(), HIT_TEXT_LIMIT + 1);
    }
}
