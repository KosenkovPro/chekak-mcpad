//! Rope backed editing buffer for normal sized files.

use ropey::{Rope, RopeSlice};

use crate::encoding::EncodingKind;
use crate::eol::LineEnding;

/// A text document held in memory as a rope.
#[derive(Debug, Clone)]
pub struct Document {
    rope: Rope,
    encoding: EncodingKind,
    line_ending: LineEnding,
}

impl Document {
    /// Builds a document from raw bytes, detecting the encoding.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let encoding = EncodingKind::detect(bytes);
        let text = encoding.decode(bytes);
        Self::from_text_with(&text, encoding)
    }

    /// Builds a document from a string.
    pub fn from_text(text: &str) -> Self {
        Self::from_text_with(text, EncodingKind::Utf8)
    }

    /// Builds a document from a string with an explicit encoding.
    pub fn from_text_with(text: &str, encoding: EncodingKind) -> Self {
        let line_ending = LineEnding::detect(text);
        Self {
            rope: Rope::from_str(text),
            encoding,
            line_ending,
        }
    }

    /// Detected or assigned encoding.
    pub fn encoding(&self) -> EncodingKind {
        self.encoding
    }

    /// Detected or assigned line ending.
    pub fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    /// Number of characters.
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Whether the document is empty.
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    /// Number of lines. An empty document still reports one (empty) line.
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    /// Text of a single line, without the line ending.
    pub fn line(&self, index: usize) -> RopeSlice<'_> {
        self.rope.line(index)
    }

    /// The whole text.
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    /// Serialises the document back to bytes in its own encoding and line ending.
    pub fn to_bytes(&self) -> Vec<u8> {
        let text = LineEnding::convert(&self.rope.to_string(), self.line_ending);
        self.encoding.encode(&text)
    }

    /// Inserts text at a character position.
    pub fn insert(&mut self, char_index: usize, text: &str) {
        self.rope.insert(char_index, text);
    }

    /// Removes a character range.
    pub fn remove(&mut self, char_range: std::ops::Range<usize>) {
        self.rope.remove(char_range);
    }

    /// Replaces the whole content.
    pub fn set_text(&mut self, text: &str) {
        self.rope = Rope::from_str(text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_text() {
        let doc = Document::from_text("привет\nмир\n");
        assert_eq!(doc.text(), "привет\nмир\n");
        assert_eq!(doc.len_lines(), 3);
    }

    #[test]
    fn detects_encoding_and_line_ending() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"a\r\nb");
        let doc = Document::from_bytes(&bytes);
        assert_eq!(doc.encoding(), EncodingKind::Utf8Bom);
        assert_eq!(doc.line_ending(), LineEnding::Crlf);
    }

    #[test]
    fn serialises_with_the_original_encoding() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice("hello".as_bytes());
        let doc = Document::from_bytes(&bytes);
        assert_eq!(doc.to_bytes(), bytes);
    }

    #[test]
    fn preserves_crlf_on_serialisation() {
        let doc = Document::from_text("a\r\nb\r\n");
        assert_eq!(doc.to_bytes(), b"a\r\nb\r\n");
    }

    #[test]
    fn editing_works() {
        let mut doc = Document::from_text("hello");
        doc.insert(5, " world");
        assert_eq!(doc.text(), "hello world");
        doc.remove(0..6);
        assert_eq!(doc.text(), "world");
    }

    #[test]
    fn line_access_is_indexed() {
        let doc = Document::from_text("first\nsecond\nthird");
        assert_eq!(doc.line(0), "first\n");
        assert_eq!(doc.line(1), "second\n");
        assert_eq!(doc.line(2), "third");
    }
}
