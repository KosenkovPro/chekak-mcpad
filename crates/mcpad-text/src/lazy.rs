//! Lazy access to huge files.
//!
//! Opening a multi gigabyte log must not allocate a buffer for it. `LargeFile`
//! scans the file once to build a sparse line index (one entry every
//! `SPARSE_STEP` lines) and afterwards reads only the requested line window
//! from disk.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::encoding::EncodingKind;
use mcpad_core::error::{Error, Result};

/// Size of the chunks used while scanning and reading.
const CHUNK_SIZE: usize = 1 << 20;

/// One index entry per this many lines.
const SPARSE_STEP: usize = 1024;

/// A file opened for random access by line number.
pub struct LargeFile {
    path: PathBuf,
    encoding: EncodingKind,
    byte_len: u64,
    line_count: usize,
    /// Byte offset of the first line of every `SPARSE_STEP` block.
    index: Vec<u64>,
}

impl LargeFile {
    /// Scans the file and builds the sparse line index.
    pub fn open(path: &Path) -> Result<Self> {
        let mut file = File::open(path).map_err(|e| Error::io(path, e))?;
        let byte_len = file.metadata().map_err(|e| Error::io(path, e))?.len();

        let mut index = vec![0u64];
        let mut lines = 0usize;
        let mut offset = 0u64;
        let mut buffer = vec![0u8; CHUNK_SIZE];
        let mut last_byte = 0u8;

        loop {
            let read = file.read(&mut buffer).map_err(|e| Error::io(path, e))?;
            if read == 0 {
                break;
            }
            for (index_in_chunk, byte) in buffer.iter().enumerate().take(read) {
                if *byte == b'\n' {
                    lines += 1;
                    if lines % SPARSE_STEP == 0 {
                        index.push(offset + index_in_chunk as u64 + 1);
                    }
                }
            }
            last_byte = buffer[read - 1];
            offset += read as u64;
        }

        // A file that does not end with a newline still has a last, partial line.
        let line_count = if byte_len > 0 && last_byte != b'\n' {
            lines + 1
        } else {
            lines
        };

        // The encoding is detected from the head of the file only.
        let mut head = vec![0u8; 4096.min(byte_len as usize)];
        if !head.is_empty() {
            let mut handle = File::open(path).map_err(|e| Error::io(path, e))?;
            handle
                .read_exact(&mut head)
                .map_err(|e| Error::io(path, e))?;
        }
        let encoding = EncodingKind::detect(&head);

        Ok(Self {
            path: path.to_path_buf(),
            encoding,
            byte_len,
            line_count,
            index,
        })
    }

    /// Path of the file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Detected encoding.
    pub fn encoding(&self) -> EncodingKind {
        self.encoding
    }

    /// Size of the file in bytes.
    pub fn byte_len(&self) -> u64 {
        self.byte_len
    }

    /// Number of lines in the file.
    pub fn line_count(&self) -> usize {
        self.line_count
    }

    /// Reads `count` lines starting at `start`.
    pub fn read_lines(&self, start: usize, count: usize) -> Result<String> {
        let end = start.saturating_add(count).min(self.line_count);
        if start >= end {
            return Ok(String::new());
        }

        let begin = self.offset_of_line(start)?;
        let finish = if end < self.line_count {
            self.offset_of_line(end)?
        } else {
            self.byte_len
        };

        let mut file = File::open(&self.path).map_err(|e| Error::io(&self.path, e))?;
        file.seek(SeekFrom::Start(begin))
            .map_err(|e| Error::io(&self.path, e))?;

        let mut buffer = Vec::with_capacity((finish - begin) as usize);
        file.by_ref()
            .take(finish - begin)
            .read_to_end(&mut buffer)
            .map_err(|e| Error::io(&self.path, e))?;

        Ok(self.encoding.decode(&buffer))
    }

    /// Byte offset where the given line starts.
    fn offset_of_line(&self, line: usize) -> Result<u64> {
        if line == 0 {
            return Ok(0);
        }

        let sparse_index = (line / SPARSE_STEP).min(self.index.len().saturating_sub(1));
        let mut current_line = sparse_index * SPARSE_STEP;
        let mut offset = self.index[sparse_index];

        if current_line >= line {
            return Ok(offset);
        }

        let mut file = File::open(&self.path).map_err(|e| Error::io(&self.path, e))?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| Error::io(&self.path, e))?;

        let mut buffer = vec![0u8; 64 * 1024];
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|e| Error::io(&self.path, e))?;
            if read == 0 {
                break;
            }
            for (index_in_chunk, byte) in buffer.iter().enumerate().take(read) {
                if *byte == b'\n' {
                    current_line += 1;
                    if current_line == line {
                        return Ok(offset + index_in_chunk as u64 + 1);
                    }
                }
            }
            offset += read as u64;
        }

        Ok(self.byte_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_lines(path: &Path, count: usize) {
        let mut content = String::new();
        for i in 0..count {
            content.push_str(&format!("line {i}\n"));
        }
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn counts_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("log.txt");
        write_lines(&path, 100);
        let file = LargeFile::open(&path).unwrap();
        assert_eq!(file.line_count(), 100);
    }

    #[test]
    fn counts_a_trailing_partial_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("log.txt");
        std::fs::write(&path, "a\nb\nc").unwrap();
        let file = LargeFile::open(&path).unwrap();
        assert_eq!(file.line_count(), 3);
    }

    #[test]
    fn reads_a_window_of_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("log.txt");
        write_lines(&path, 50);
        let file = LargeFile::open(&path).unwrap();
        let window = file.read_lines(10, 3).unwrap();
        assert_eq!(window, "line 10\nline 11\nline 12\n");
    }

    #[test]
    fn reads_the_last_line_without_trailing_newline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("log.txt");
        std::fs::write(&path, "a\nb\nc").unwrap();
        let file = LargeFile::open(&path).unwrap();
        assert_eq!(file.read_lines(2, 1).unwrap(), "c");
    }

    #[test]
    fn reads_beyond_the_sparse_index_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("log.txt");
        write_lines(&path, 5000);
        let file = LargeFile::open(&path).unwrap();
        assert_eq!(file.line_count(), 5000);
        let window = file.read_lines(4090, 3).unwrap();
        assert_eq!(window, "line 4090\nline 4091\nline 4092\n");
    }

    #[test]
    fn out_of_range_windows_are_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("log.txt");
        write_lines(&path, 5);
        let file = LargeFile::open(&path).unwrap();
        assert_eq!(file.read_lines(100, 10).unwrap(), "");
        assert_eq!(file.read_lines(0, 0).unwrap(), "");
    }

    #[test]
    fn detects_the_encoding_of_a_large_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("log.txt");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice("привет\n".repeat(10).as_bytes());
        std::fs::write(&path, bytes).unwrap();
        let file = LargeFile::open(&path).unwrap();
        assert_eq!(file.encoding(), EncodingKind::Utf8Bom);
        assert_eq!(file.read_lines(0, 1).unwrap(), "привет\n");
    }
}
