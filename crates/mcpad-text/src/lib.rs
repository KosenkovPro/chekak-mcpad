//! Text primitives for MCPad.
//!
//! * [`EncodingKind`] detects and converts the encodings the editor supports.
//! * [`LineEnding`] detects and converts `LF` / `CRLF`.
//! * [`Document`] is the rope backed buffer used for normal sized files.
//! * [`LargeFile`] opens huge files lazily: only the requested line window is
//!   read from disk, the file is never loaded as a whole.

pub mod document;
pub mod encoding;
pub mod eol;
pub mod lazy;

pub use document::Document;
pub use encoding::EncodingKind;
pub use eol::LineEnding;
pub use lazy::LargeFile;

pub use mcpad_core::fs::atomic_write;
