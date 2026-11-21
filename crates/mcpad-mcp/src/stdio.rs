//! The stdio transport of the protocol.
//!
//! Every line of the input is a message of the client, every line of the
//! output is an answer. Nothing but the protocol may be written to the output,
//! the log goes to the error output.

use std::io::{BufRead, Write};

use mcpad_core::{Error, Result};

use crate::backend::Backend;
use crate::protocol;
use crate::server::McpServer;

/// Serves the protocol on the standard input and output.
pub fn serve<B: Backend>(
    server: &McpServer<B>,
    input: impl BufRead,
    output: &mut impl Write,
) -> Result<()> {
    for line in input.lines() {
        let line = line.map_err(Error::IoBare)?;
        let trimmed = protocol::strip_bom(line.trim());
        if trimmed.is_empty() {
            continue;
        }

        if let Some(answer) = server.handle_line(trimmed) {
            writeln!(output, "{answer}").map_err(Error::IoBare)?;
            output.flush().map_err(Error::IoBare)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use serde_json::{json, Value};

    use super::*;
    use crate::gate::DenyGate;
    use crate::vault::VaultBackend;
    fn server_with_notes() -> (tempfile::TempDir, McpServer<VaultBackend>) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("notes")).unwrap();
        std::fs::write(dir.path().join("notes/a.md"), "# A\n").unwrap();
        let backend = VaultBackend::open(dir.path()).unwrap();
        let server = McpServer::new(backend, std::sync::Arc::new(DenyGate));
        (dir, server)
    }

    #[test]
    fn the_answers_come_back_line_by_line() {
        let (_dir, server) = server_with_notes();
        let input = Cursor::new(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-06-18\"}}\n\
             {\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n\
             {\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\n",
        );
        let mut output: Vec<u8> = Vec::new();

        serve(&server, input, &mut output).unwrap();

        let text = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "a notification has no answer");

        let first: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first["id"], 1);
        assert_eq!(first["result"]["protocolVersion"], "2025-06-18");

        let second: Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(second["id"], 2);
        assert_eq!(second["result"]["tools"].as_array().unwrap().len(), 7);
    }

    #[test]
    fn a_broken_line_does_not_stop_the_server() {
        let (_dir, server) = server_with_notes();
        let input = Cursor::new("not json\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n");
        let mut output: Vec<u8> = Vec::new();

        serve(&server, input, &mut output).unwrap();

        let text = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);

        let first: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first["error"]["code"], protocol::PARSE_ERROR);

        let second: Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(second["result"], json!({}));
    }

    #[test]
    fn empty_lines_are_skipped() {
        let (_dir, server) = server_with_notes();
        let input = Cursor::new("\n   \n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n");
        let mut output: Vec<u8> = Vec::new();

        serve(&server, input, &mut output).unwrap();
        assert_eq!(String::from_utf8(output).unwrap().lines().count(), 1);
    }

    #[test]
    fn a_byte_order_mark_in_front_of_a_message_is_tolerated() {
        let (_dir, server) = server_with_notes();
        let input = Cursor::new("\u{feff}{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n");
        let mut output: Vec<u8> = Vec::new();

        serve(&server, input, &mut output).unwrap();

        let text = String::from_utf8(output).unwrap();
        let answer: Value = serde_json::from_str(text.trim()).unwrap();
        assert_eq!(answer["result"], json!({}));
    }
}
