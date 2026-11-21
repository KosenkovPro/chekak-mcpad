//! Error type shared by every MCPad crate.

use std::path::PathBuf;

use serde::{ser::SerializeStruct, Serialize, Serializer};

/// Errors produced by the core domain model.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An I/O failure that happened while working with a specific path.
    #[error("i/o error at `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// An I/O failure without a meaningful path attached.
    #[error("i/o error: {0}")]
    IoBare(#[from] std::io::Error),

    /// Malformed JSON in a configuration or theme file.
    #[error("malformed json: {0}")]
    Json(#[from] serde_json::Error),

    /// The configuration file exists but does not make sense.
    #[error("invalid configuration: {0}")]
    Config(String),

    /// The vault root exists but is not a directory.
    #[error("`{0}` is not a directory")]
    NotADirectory(PathBuf),

    /// The platform did not report a usable home directory.
    #[error("cannot locate a writable home directory for MCPad data")]
    NoHomeDir,

    /// A `.mcpadignore` pattern could not be compiled.
    #[error("invalid ignore pattern `{pattern}`: {message}")]
    Glob { pattern: String, message: String },

    /// Anything that does not deserve its own variant.
    #[error("{0}")]
    Message(String),
}

impl Error {
    /// Builds an [`Error::Io`] for the given path and source error.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Error::Io {
            path: path.into(),
            source,
        }
    }

    /// Builds an [`Error::Message`].
    pub fn message(message: impl Into<String>) -> Self {
        Error::Message(message.into())
    }

    /// A short machine readable name of the error kind.
    pub fn kind(&self) -> &'static str {
        match self {
            Error::Io { .. } => "io",
            Error::IoBare(_) => "io",
            Error::Json(_) => "json",
            Error::Config(_) => "config",
            Error::NotADirectory(_) => "not-a-directory",
            Error::NoHomeDir => "no-home-dir",
            Error::Glob { .. } => "glob",
            Error::Message(_) => "message",
        }
    }
}

/// The error is sent to the frontend as a small object, so the user interface
/// can show a meaningful message instead of a generic failure.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("McpadError", 2)?;
        state.serialize_field("kind", self.kind())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

/// Convenience alias used across the workspace.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_errors_keep_their_path() {
        let err = Error::io("/tmp/x", std::io::Error::other("boom"));
        let rendered = err.to_string();
        assert!(rendered.contains("/tmp/x"), "{rendered}");
        assert!(rendered.contains("boom"), "{rendered}");
    }

    #[test]
    fn errors_can_be_sent_to_the_frontend() {
        let err = Error::message("нет такого файла");
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["kind"], "message");
        assert_eq!(json["message"], "нет такого файла");
    }

    #[test]
    fn io_errors_report_their_kind() {
        let err = Error::io("/tmp/x", std::io::Error::other("boom"));
        assert_eq!(err.kind(), "io");
    }
}
