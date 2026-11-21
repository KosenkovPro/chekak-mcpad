//! The MCP server: it turns the messages of a client into calls of a backend.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use mcpad_app::SearchOptions;
use serde_json::{json, Value};

use crate::backend::{Backend, DEFAULT_READ_LINES};
use crate::gate::{Decision, Gate, WriteRequest, PREVIEW_LIMIT};
use crate::protocol::{self, CallToolResult, Content, ListResourcesResult, Resource, RpcError};
use crate::tools;

/// The name of the server.
pub const SERVER_NAME: &str = "mcpad";

/// The version of the server.
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The hint the server gives to a model on how to work with the vault.
pub const INSTRUCTIONS: &str = "MCPad is a local knowledge base: a plain directory of notes. \
Start with `vault_info` and `list_files`, then read with `read_file` and look for \
something with `search`. Paths are relative to the root of the vault and use forward \
slashes. The tools that change the vault (`write_file` and `delete_file`) need a \
confirmation of the user and fail when the change is refused.";

/// The number of the requests to confirm, used for the identifiers.
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// The server itself.
pub struct McpServer<B: Backend> {
    backend: B,
    gate: Arc<dyn Gate>,
    state: Mutex<State>,
}

/// What the server knows about the client.
struct State {
    protocol_version: String,
}

impl<B: Backend> McpServer<B> {
    /// Builds a server on top of a backend.
    pub fn new(backend: B, gate: Arc<dyn Gate>) -> Self {
        Self {
            backend,
            gate,
            state: Mutex::new(State {
                protocol_version: protocol::LATEST_PROTOCOL_VERSION.to_string(),
            }),
        }
    }

    /// The backend behind the server.
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// The revision of the protocol that was agreed on.
    pub fn protocol_version(&self) -> String {
        lock(&self.state).protocol_version.clone()
    }

    /// Remembers the revision the client named in the header of a request.
    pub fn set_protocol_version(&self, version: &str) {
        if protocol::SUPPORTED_PROTOCOL_VERSIONS.contains(&version) {
            lock(&self.state).protocol_version = version.to_string();
        }
    }

    /// Handles one message of a client and returns the answer, if there is one.
    ///
    /// A notification has no answer, therefore `None` is returned for it.
    pub fn handle(&self, message: Value) -> Option<Value> {
        if message.is_array() {
            let error = RpcError::new(
                protocol::INVALID_REQUEST,
                "batches of requests are not supported",
            );
            return Some(protocol::failure(Value::Null, &error));
        }

        let request = match protocol::read_request(&message) {
            Ok(request) => request,
            Err(error) => {
                let id = message.get("id").cloned().unwrap_or(Value::Null);
                return Some(protocol::failure(id, &error));
            }
        };

        let id = request.id.clone();
        let answer = match self.dispatch(&request) {
            Ok(result) => protocol::success(id.clone().unwrap_or(Value::Null), result),
            Err(error) => protocol::failure(id.clone().unwrap_or(Value::Null), &error),
        };

        id.map(|_| answer)
    }

    /// Handles one line of text and returns the answer as a line of text.
    pub fn handle_line(&self, line: &str) -> Option<String> {
        let message: Value = match serde_json::from_str(line) {
            Ok(message) => message,
            Err(error) => {
                let error = RpcError::new(
                    protocol::PARSE_ERROR,
                    format!("the message cannot be parsed: {error}"),
                );
                let answer = protocol::failure(Value::Null, &error);
                return serde_json::to_string(&answer).ok();
            }
        };

        let answer = self.handle(message)?;
        serde_json::to_string(&answer).ok()
    }

    /// Calls the method of a request.
    fn dispatch(&self, request: &protocol::Request) -> Result<Value, RpcError> {
        match request.method.as_str() {
            "initialize" => Ok(self.initialize(&request.params)),
            "notifications/initialized" | "initialized" => Ok(Value::Null),
            "notifications/cancelled" => Ok(Value::Null),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(self.list_tools()),
            "tools/call" => {
                let result = self.call_tool(&request.params);
                Ok(serde_json::to_value(result).unwrap_or_else(|_| json!({ "content": [] })))
            }
            "resources/list" => Ok(self.list_resources()),
            "resources/read" => self.read_resource(&request.params),
            "prompts/list" => Ok(json!({ "prompts": [] })),
            other => Err(RpcError::new(
                protocol::METHOD_NOT_FOUND,
                format!("the method `{other}` is unknown"),
            )),
        }
    }

    /// The answer of the `initialize` method.
    fn initialize(&self, params: &Value) -> Value {
        let requested = params.get("protocolVersion").and_then(Value::as_str);
        let version = protocol::negotiate(requested);
        lock(&self.state).protocol_version = version.clone();

        json!({
            "protocolVersion": version,
            "capabilities": {
                "tools": { "listChanged": false },
                "resources": { "subscribe": false, "listChanged": false }
            },
            "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION },
            "instructions": INSTRUCTIONS
        })
    }

    /// The answer of the `tools/list` method.
    fn list_tools(&self) -> Value {
        let list = protocol::ListToolsResult {
            tools: tools::catalogue(),
        };
        serde_json::to_value(list).unwrap_or_else(|_| json!({ "tools": [] }))
    }

    /// The answer of the `resources/list` method.
    fn list_resources(&self) -> Value {
        let files = match self.backend.list_files() {
            Ok(files) => files,
            Err(error) => return json!({ "resources": [], "error": error.to_string() }),
        };

        let resources: Vec<Resource> = files
            .iter()
            .map(|file| Resource {
                uri: protocol::resource_uri(&file.path),
                name: file.name.clone(),
                mime_type: Some(protocol::mime_type_of(&file.path).to_string()),
                description: None,
            })
            .collect();

        let list = ListResourcesResult { resources };
        serde_json::to_value(list).unwrap_or_else(|_| json!({ "resources": [] }))
    }

    /// The answer of the `resources/read` method.
    fn read_resource(&self, params: &Value) -> Result<Value, RpcError> {
        let uri = params
            .get("uri")
            .and_then(Value::as_str)
            .ok_or_else(|| RpcError::invalid_params("the `uri` of the resource is missing"))?;
        let path = protocol::path_of_resource(uri)?;

        let slice = self
            .backend
            .read_file_full(&path)
            .map_err(|error| RpcError::internal(error.to_string()))?;

        Ok(json!({
            "contents": [{
                "uri": uri,
                "mimeType": protocol::mime_type_of(&path),
                "text": slice.text
            }]
        }))
    }

    /// The answer of the `tools/call` method.
    fn call_tool(&self, params: &Value) -> CallToolResult {
        let name = match protocol::tool_name_of(params) {
            Ok(name) => name,
            Err(error) => return tool_error(error.message),
        };
        let arguments = protocol::arguments_of(params);

        match name.as_str() {
            tools::LIST_FILES => self.tool_list_files(),
            tools::READ_FILE => self.tool_read_file(&arguments),
            tools::READ_FILE_FULL => self.tool_read_file_full(&arguments),
            tools::SEARCH => self.tool_search(&arguments),
            tools::VAULT_INFO => self.tool_vault_info(),
            tools::WRITE_FILE => self.tool_write_file(&arguments),
            tools::DELETE_FILE => self.tool_delete_file(&arguments),
            other => tool_error(format!(
                "the tool `{other}` is unknown, the tools of the server are: {}",
                tools::catalogue()
                    .iter()
                    .map(|tool| tool.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }

    /// The tool that lists the files of the vault.
    fn tool_list_files(&self) -> CallToolResult {
        let files = match self.backend.list_files() {
            Ok(files) => files,
            Err(error) => return tool_error(error.to_string()),
        };

        if files.is_empty() {
            return self.tool_ok("The vault has no files yet.", Some(json!({ "files": [] })));
        }

        let text = files
            .iter()
            .map(|file| format!("{} ({} bytes, {})", file.path, file.size, file.kind))
            .collect::<Vec<_>>()
            .join("\n");

        self.tool_ok(text, Some(json!({ "files": files })))
    }

    /// The tool that reads a part of a file.
    fn tool_read_file(&self, arguments: &Value) -> CallToolResult {
        let path = match argument(arguments, "path") {
            Ok(path) => path,
            Err(result) => return result,
        };
        let start_line = match optional_number(arguments, "start_line") {
            Ok(value) => value.unwrap_or(1).max(1) - 1,
            Err(result) => return result,
        };
        let max_lines = match optional_number(arguments, "max_lines") {
            Ok(value) => value.unwrap_or(DEFAULT_READ_LINES),
            Err(result) => return result,
        };

        let slice = match self.backend.read_file(&path, start_line, max_lines) {
            Ok(slice) => slice,
            Err(error) => return tool_error(error.to_string()),
        };

        self.tool_ok(
            slice.text.clone(),
            Some(json!({
                "path": slice.path,
                "encoding": slice.encoding,
                "lineEnding": slice.line_ending,
                "startLine": slice.start_line,
                "returnedLines": slice.returned_lines,
                "totalLines": slice.total_lines,
                "truncated": slice.truncated
            })),
        )
    }

    /// The tool that reads a file completely.
    fn tool_read_file_full(&self, arguments: &Value) -> CallToolResult {
        let path = match argument(arguments, "path") {
            Ok(path) => path,
            Err(result) => return result,
        };

        let slice = match self.backend.read_file_full(&path) {
            Ok(slice) => slice,
            Err(error) => return tool_error(error.to_string()),
        };

        self.tool_ok(
            slice.text.clone(),
            Some(json!({
                "path": slice.path,
                "encoding": slice.encoding,
                "lineEnding": slice.line_ending,
                "startLine": slice.start_line,
                "returnedLines": slice.returned_lines,
                "totalLines": slice.total_lines,
                "truncated": slice.truncated
            })),
        )
    }

    /// The tool that searches the text of the vault.
    fn tool_search(&self, arguments: &Value) -> CallToolResult {
        let query = match argument(arguments, "query") {
            Ok(query) => query,
            Err(result) => return result,
        };
        let regex = match optional_flag(arguments, "regex") {
            Ok(value) => value.unwrap_or(false),
            Err(result) => return result,
        };
        let case_sensitive = match optional_flag(arguments, "case_sensitive") {
            Ok(value) => value.unwrap_or(false),
            Err(result) => return result,
        };
        let prefix = match optional_text(arguments, "path") {
            Ok(value) => value,
            Err(result) => return result,
        };
        let max_results = match optional_number(arguments, "max_results") {
            Ok(value) => value.unwrap_or(SearchOptions::default().max_results),
            Err(result) => return result,
        };

        let options = SearchOptions {
            regex,
            case_sensitive,
            prefix,
            max_results,
            ..SearchOptions::default()
        };

        let hits = match self.backend.search(&query, &options) {
            Ok(hits) => hits,
            Err(error) => return tool_error(error.to_string()),
        };

        if hits.is_empty() {
            return self.tool_ok(
                format!("Nothing matches `{query}`."),
                Some(json!({ "hits": [] })),
            );
        }

        let text = hits
            .iter()
            .map(|hit| format!("{}:{}: {}", hit.path, hit.line, hit.text))
            .collect::<Vec<_>>()
            .join("\n");

        self.tool_ok(text, Some(json!({ "hits": hits })))
    }

    /// The tool that tells about the vault.
    fn tool_vault_info(&self) -> CallToolResult {
        let files = match self.backend.list_files() {
            Ok(files) => files,
            Err(error) => return tool_error(error.to_string()),
        };
        let bytes: u64 = files.iter().map(|file| file.size).sum();

        let text = format!(
            "The root of the vault is {}.\nIt holds {} files with {} bytes in total.",
            self.backend.root().display(),
            files.len(),
            bytes
        );

        self.tool_ok(
            text,
            Some(json!({
                "root": self.backend.root().display().to_string(),
                "files": files.len(),
                "bytes": bytes
            })),
        )
    }

    /// The tool that writes a file of the vault.
    fn tool_write_file(&self, arguments: &Value) -> CallToolResult {
        let path = match argument(arguments, "path") {
            Ok(path) => path,
            Err(result) => return result,
        };
        let text = match argument(arguments, "text") {
            Ok(text) => text,
            Err(result) => return result,
        };
        let encoding = match optional_text(arguments, "encoding") {
            Ok(value) => value,
            Err(result) => return result,
        };
        let line_ending = match optional_text(arguments, "line_ending") {
            Ok(value) => value,
            Err(result) => return result,
        };

        let request = WriteRequest::new(
            next_request_id(),
            tools::WRITE_FILE,
            &path,
            format!("write {} bytes to `{path}`", text.len()),
            Some(preview_of(&text)),
        );

        match self.gate.decide(request) {
            Decision::Allow => match self.backend.write_file(
                &path,
                &text,
                encoding.as_deref(),
                line_ending.as_deref(),
            ) {
                Ok(()) => self.tool_ok(
                    format!("The file `{path}` is written."),
                    Some(json!({ "path": path, "bytes": text.len() })),
                ),
                Err(error) => tool_error(error.to_string()),
            },
            Decision::Deny(reason) => {
                tool_error(format!("The file `{path}` is not written: {reason}"))
            }
        }
    }

    /// The tool that removes a file of the vault.
    fn tool_delete_file(&self, arguments: &Value) -> CallToolResult {
        let path = match argument(arguments, "path") {
            Ok(path) => path,
            Err(result) => return result,
        };

        let request = WriteRequest::new(
            next_request_id(),
            tools::DELETE_FILE,
            &path,
            format!("delete `{path}`"),
            None,
        );

        match self.gate.decide(request) {
            Decision::Allow => match self.backend.delete_file(&path) {
                Ok(()) => self.tool_ok(
                    format!("The file `{path}` is removed."),
                    Some(json!({ "path": path })),
                ),
                Err(error) => tool_error(error.to_string()),
            },
            Decision::Deny(reason) => {
                tool_error(format!("The file `{path}` is not removed: {reason}"))
            }
        }
    }

    /// Builds a successful answer of a tool.
    fn tool_ok(&self, text: impl Into<String>, structured: Option<Value>) -> CallToolResult {
        let structured = if protocol::supports_structured_output(&self.protocol_version()) {
            structured
        } else {
            None
        };

        CallToolResult {
            content: vec![Content::Text { text: text.into() }],
            is_error: None,
            structured_content: structured,
        }
    }
}

/// Builds a failed answer of a tool.
fn tool_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult {
        content: vec![Content::Text {
            text: message.into(),
        }],
        is_error: Some(true),
        structured_content: None,
    }
}

/// Reads a required argument.
fn argument(arguments: &Value, name: &str) -> Result<String, CallToolResult> {
    protocol::string_member(arguments, name).map_err(|error| tool_error(error.message))
}

/// Reads an optional text argument.
fn optional_text(arguments: &Value, name: &str) -> Result<Option<String>, CallToolResult> {
    protocol::optional_string(arguments, name).map_err(|error| tool_error(error.message))
}

/// Reads an optional number argument.
fn optional_number(arguments: &Value, name: &str) -> Result<Option<usize>, CallToolResult> {
    protocol::optional_usize(arguments, name).map_err(|error| tool_error(error.message))
}

/// Reads an optional flag argument.
fn optional_flag(arguments: &Value, name: &str) -> Result<Option<bool>, CallToolResult> {
    protocol::optional_bool(arguments, name).map_err(|error| tool_error(error.message))
}

/// Builds the beginning of a text, for the preview of a change.
fn preview_of(text: &str) -> String {
    let head: String = text.chars().take(PREVIEW_LIMIT).collect();
    if text.chars().count() > PREVIEW_LIMIT {
        format!("{head}…")
    } else {
        head
    }
}

/// The identifier of the next request to confirm.
fn next_request_id() -> String {
    let number = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("change-{number}")
}

/// Locks a mutex and ignores a poisoned lock.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;
    use std::time::Duration;

    use super::*;
    use crate::gate::{AllowGate, ChannelGate, DenyGate};
    use crate::vault::VaultBackend;

    fn vault_with_notes() -> (tempfile::TempDir, VaultBackend) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("notes")).unwrap();
        std::fs::write(dir.path().join("notes/a.md"), "# A\n\nпервая\n").unwrap();
        let backend = VaultBackend::open(dir.path()).unwrap();
        (dir, backend)
    }

    fn server(backend: VaultBackend, gate: Arc<dyn Gate>) -> McpServer<VaultBackend> {
        McpServer::new(backend, gate)
    }

    fn call(server: &McpServer<VaultBackend>, name: &str, arguments: Value) -> Value {
        let message = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments }
        });
        let answer = server.handle(message).unwrap();
        answer["result"].clone()
    }

    #[test]
    fn the_handshake_names_the_server_and_the_revision() {
        let (_dir, backend) = vault_with_notes();
        let server = server(backend, Arc::new(DenyGate));

        let answer = server
            .handle(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": { "protocolVersion": "2025-03-26" }
            }))
            .unwrap();

        assert_eq!(answer["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(answer["result"]["serverInfo"]["name"], "mcpad");
        assert_eq!(
            answer["result"]["capabilities"]["tools"]["listChanged"],
            false
        );
        assert!(answer["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("read_file"));
    }

    #[test]
    fn an_unknown_revision_is_answered_with_a_known_one() {
        let (_dir, backend) = vault_with_notes();
        let server = server(backend, Arc::new(DenyGate));

        let answer = server
            .handle(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": { "protocolVersion": "1999-01-01" }
            }))
            .unwrap();

        assert_eq!(
            answer["result"]["protocolVersion"],
            protocol::LATEST_PROTOCOL_VERSION
        );
    }

    #[test]
    fn the_tools_are_listed_with_their_schemas() {
        let (_dir, backend) = vault_with_notes();
        let server = server(backend, Arc::new(DenyGate));

        let answer = server
            .handle(json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }))
            .unwrap();

        let tools = answer["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 7);
        assert!(tools
            .iter()
            .all(|tool| tool["inputSchema"]["type"] == "object"));
    }

    #[test]
    fn a_notification_has_no_answer() {
        let (_dir, backend) = vault_with_notes();
        let server = server(backend, Arc::new(DenyGate));

        let answer = server.handle(json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }));
        assert!(answer.is_none());
    }

    #[test]
    fn an_unknown_method_is_refused() {
        let (_dir, backend) = vault_with_notes();
        let server = server(backend, Arc::new(DenyGate));

        let answer = server
            .handle(json!({ "jsonrpc": "2.0", "id": 1, "method": "dance" }))
            .unwrap();

        assert_eq!(answer["error"]["code"], protocol::METHOD_NOT_FOUND);
    }

    #[test]
    fn a_broken_message_is_refused() {
        let (_dir, backend) = vault_with_notes();
        let server = server(backend, Arc::new(DenyGate));

        let answer = server.handle_line("not json at all").unwrap();
        let parsed: Value = serde_json::from_str(&answer).unwrap();
        assert_eq!(parsed["error"]["code"], protocol::PARSE_ERROR);
    }

    #[test]
    fn the_files_of_the_vault_are_listed() {
        let (_dir, backend) = vault_with_notes();
        let server = server(backend, Arc::new(DenyGate));

        let result = call(&server, "list_files", json!({}));
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("notes/a.md"));
        assert_eq!(
            result["structuredContent"]["files"][0]["path"],
            "notes/a.md"
        );
    }

    #[test]
    fn a_file_is_read_with_its_text() {
        let (_dir, backend) = vault_with_notes();
        let server = server(backend, Arc::new(DenyGate));

        let result = call(&server, "read_file", json!({ "path": "notes/a.md" }));
        assert_eq!(result["content"][0]["text"], "# A\n\nпервая\n");
        assert_eq!(result["structuredContent"]["totalLines"], 3);
    }

    #[test]
    fn a_part_of_a_file_is_read() {
        let (_dir, backend) = vault_with_notes();
        let server = server(backend, Arc::new(DenyGate));

        let result = call(
            &server,
            "read_file",
            json!({ "path": "notes/a.md", "start_line": 3, "max_lines": 1 }),
        );
        assert_eq!(result["content"][0]["text"], "первая\n");
        assert_eq!(result["structuredContent"]["truncated"], false);
    }

    #[test]
    fn a_missing_argument_is_reported_inside_the_answer() {
        let (_dir, backend) = vault_with_notes();
        let server = server(backend, Arc::new(DenyGate));

        let result = call(&server, "read_file", json!({}));
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("`path` is missing"));
    }

    #[test]
    fn the_vault_is_searched() {
        let (_dir, backend) = vault_with_notes();
        let server = server(backend, Arc::new(DenyGate));

        let result = call(&server, "search", json!({ "query": "первая" }));
        assert_eq!(result["structuredContent"]["hits"][0]["path"], "notes/a.md");
        assert_eq!(result["structuredContent"]["hits"][0]["line"], 3);
    }

    #[test]
    fn the_read_only_mode_refuses_to_write() {
        let (dir, backend) = vault_with_notes();
        let server = server(backend, Arc::new(DenyGate));

        let result = call(
            &server,
            "write_file",
            json!({ "path": "notes/b.md", "text": "новая\n" }),
        );

        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("read only"));
        assert!(!dir.path().join("notes/b.md").exists());
    }

    #[test]
    fn a_file_is_written_when_the_change_is_allowed() {
        let (dir, backend) = vault_with_notes();
        let server = server(backend, Arc::new(AllowGate));

        let result = call(
            &server,
            "write_file",
            json!({ "path": "notes/b.md", "text": "новая\n" }),
        );

        assert!(result["isError"].is_null());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("notes/b.md")).unwrap(),
            "новая\n"
        );
    }

    #[test]
    fn a_file_is_written_when_the_user_confirms_the_change() {
        let (dir, backend) = vault_with_notes();
        let gate = Arc::new(ChannelGate::new(Duration::from_secs(5)));
        let (sender, receiver) = channel();
        gate.set_listener(sender);
        let server = server(backend, gate.clone());

        let answer = std::thread::spawn(move || {
            call(
                &server,
                "write_file",
                json!({ "path": "notes/c.md", "text": "подтверждённая\n" }),
            )
        });

        let request = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(request.path, "notes/c.md");
        assert_eq!(request.tool, "write_file");
        assert!(request.preview.unwrap().contains("подтверждённая"));
        assert!(!dir.path().join("notes/c.md").exists());

        assert!(gate.resolve(&request.id, Decision::Allow));

        let result = answer.join().unwrap();
        assert!(result["isError"].is_null());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("notes/c.md")).unwrap(),
            "подтверждённая\n"
        );
    }

    #[test]
    fn a_file_is_kept_when_the_user_refuses_the_change() {
        let (dir, backend) = vault_with_notes();
        let gate = Arc::new(ChannelGate::new(Duration::from_secs(5)));
        let (sender, receiver) = channel();
        gate.set_listener(sender);
        let server = server(backend, gate.clone());

        let answer = std::thread::spawn(move || {
            call(&server, "delete_file", json!({ "path": "notes/a.md" }))
        });

        let request = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(request.tool, "delete_file");
        assert!(gate.resolve(&request.id, Decision::Deny("not now".to_string())));

        let result = answer.join().unwrap();
        assert_eq!(result["isError"], true);
        assert!(dir.path().join("notes/a.md").exists());
    }

    #[test]
    fn a_file_is_removed_when_the_change_is_allowed() {
        let (dir, backend) = vault_with_notes();
        let server = server(backend, Arc::new(AllowGate));

        let result = call(&server, "delete_file", json!({ "path": "notes/a.md" }));

        assert!(result["isError"].is_null());
        assert!(!dir.path().join("notes/a.md").exists());
    }

    #[test]
    fn the_files_of_the_vault_are_resources() {
        let (_dir, backend) = vault_with_notes();
        let server = server(backend, Arc::new(DenyGate));

        let answer = server
            .handle(json!({ "jsonrpc": "2.0", "id": 1, "method": "resources/list" }))
            .unwrap();
        assert_eq!(
            answer["result"]["resources"][0]["uri"],
            "mcpad://vault/notes/a.md"
        );

        let answer = server
            .handle(json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "resources/read",
                "params": { "uri": "mcpad://vault/notes/a.md" }
            }))
            .unwrap();
        assert_eq!(answer["result"]["contents"][0]["text"], "# A\n\nпервая\n");
    }

    #[test]
    fn an_unknown_tool_is_reported_inside_the_answer() {
        let (_dir, backend) = vault_with_notes();
        let server = server(backend, Arc::new(DenyGate));

        let result = call(&server, "dance", json!({}));
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("read_file"));
    }
}
