//! The wire format of the Model Context Protocol.
//!
//! The protocol is JSON-RPC 2.0 with a few methods of its own. The types of
//! this module mirror the specification, so a client sees exactly the fields
//! it expects and nothing else.

use serde::Serialize;
use serde_json::{json, Value};

/// The newest revision of the protocol the server understands.
pub const LATEST_PROTOCOL_VERSION: &str = "2025-11-25";

/// Every revision the server understands, oldest first.
pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] =
    &["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25"];

/// The error code of a message that cannot be parsed.
pub const PARSE_ERROR: i32 = -32700;

/// The error code of a message that is not a valid request.
pub const INVALID_REQUEST: i32 = -32600;

/// The error code of an unknown method.
pub const METHOD_NOT_FOUND: i32 = -32601;

/// The error code of arguments that do not fit the method.
pub const INVALID_PARAMS: i32 = -32602;

/// The error code of a failure inside the server.
pub const INTERNAL_ERROR: i32 = -32603;

/// Picks the revision to answer with.
///
/// The client proposes a revision in `initialize`. When the server knows it,
/// the very same revision is used, otherwise the server answers with the
/// newest revision it supports.
pub fn negotiate(requested: Option<&str>) -> String {
    match requested {
        Some(version) if SUPPORTED_PROTOCOL_VERSIONS.contains(&version) => version.to_string(),
        _ => LATEST_PROTOCOL_VERSION.to_string(),
    }
}

/// Whether a revision knows the structured answer of a tool.
pub fn supports_structured_output(version: &str) -> bool {
    version >= "2025-06-18"
}

/// An error of the protocol.
#[derive(Debug, Clone, Serialize)]
pub struct RpcError {
    /// The code of the error.
    pub code: i32,
    /// A short description of the error.
    pub message: String,
    /// Additional information, when there is any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl RpcError {
    /// Builds an error with a code and a message.
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Builds an error for arguments that do not fit.
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(INVALID_PARAMS, message)
    }

    /// Builds an error for a failure inside the server.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(INTERNAL_ERROR, message)
    }
}

/// A request that passed the basic checks of the protocol.
#[derive(Debug, Clone)]
pub struct Request {
    /// The identifier of the request, `None` for a notification.
    pub id: Option<Value>,
    /// The method to call.
    pub method: String,
    /// The parameters of the call.
    pub params: Value,
}

/// Removes the byte order mark that some clients put in front of a message.
pub fn strip_bom(line: &str) -> &str {
    line.strip_prefix('\u{feff}').unwrap_or(line)
}

/// Reads a request out of a JSON value.
pub fn read_request(value: &Value) -> Result<Request, RpcError> {
    let object = value
        .as_object()
        .ok_or_else(|| RpcError::new(INVALID_REQUEST, "a request must be a JSON object"))?;

    match object.get("jsonrpc").and_then(Value::as_str) {
        Some("2.0") => {}
        _ => {
            return Err(RpcError::new(
                INVALID_REQUEST,
                "the `jsonrpc` member must be exactly \"2.0\"",
            ))
        }
    }

    let method = object
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError::new(INVALID_REQUEST, "the `method` member is missing"))?
        .to_string();

    let id = match object.get("id") {
        None | Some(Value::Null) => None,
        Some(value @ (Value::String(_) | Value::Number(_))) => Some(value.clone()),
        Some(_) => {
            return Err(RpcError::new(
                INVALID_REQUEST,
                "the `id` member must be a string or a number",
            ))
        }
    };

    let params = object.get("params").cloned().unwrap_or(Value::Null);

    Ok(Request { id, method, params })
}

/// Builds a successful answer.
pub fn success(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// Builds a failed answer.
pub fn failure(id: Value, error: &RpcError) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": error })
}

/// The answer of the `initialize` method.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// The revision of the protocol that was agreed on.
    pub protocol_version: String,
    /// What the server can do.
    pub capabilities: Capabilities,
    /// The name and the version of the server.
    pub server_info: ServerInfo,
    /// A hint for the model on how to use the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

/// What the server can do.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    /// The tools of the server.
    pub tools: ToolsCapability,
    /// The resources of the server.
    pub resources: ResourcesCapability,
}

/// The capability of the tools.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCapability {
    /// Whether the list of the tools can change while the client is connected.
    pub list_changed: bool,
}

/// The capability of the resources.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesCapability {
    /// Whether the client may subscribe to the changes of a resource.
    pub subscribe: bool,
    /// Whether the list of the resources can change while the client is connected.
    pub list_changed: bool,
}

/// The name and the version of the server.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    /// The name of the server.
    pub name: String,
    /// The version of the server.
    pub version: String,
}

/// A tool the client may call.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// The name the tool is called by.
    pub name: String,
    /// A short title for a user interface.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// What the tool does.
    pub description: String,
    /// The schema of the arguments.
    pub input_schema: Value,
    /// Hints about the behaviour of the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,
}

/// The hints about the behaviour of a tool.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolAnnotations {
    /// The title of the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The tool only reads, it never changes anything.
    pub read_only_hint: bool,
    /// The tool can destroy something.
    pub destructive_hint: bool,
    /// Calling the tool twice changes nothing the second time.
    pub idempotent_hint: bool,
    /// The tool works with the world outside of the vault.
    pub open_world_hint: bool,
}

/// The answer of the `tools/list` method.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsResult {
    /// The tools of the server.
    pub tools: Vec<Tool>,
}

/// A piece of content of an answer.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Content {
    /// A piece of text.
    Text {
        /// The text itself.
        text: String,
    },
}

/// The answer of the `tools/call` method.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResult {
    /// The content of the answer.
    pub content: Vec<Content>,
    /// Whether the tool failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    /// The answer in a machine readable form.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<Value>,
}

/// A resource of the vault.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    /// The address of the resource.
    pub uri: String,
    /// The name of the resource.
    pub name: String,
    /// The kind of the content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// A short description of the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// The answer of the `resources/list` method.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourcesResult {
    /// The resources of the vault.
    pub resources: Vec<Resource>,
}

/// The content of a resource.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceContents {
    /// The address of the resource.
    pub uri: String,
    /// The kind of the content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// The content itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// The answer of the `resources/read` method.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceResult {
    /// The content of the resource.
    pub contents: Vec<ResourceContents>,
}

/// Reads a string member of an object.
pub fn string_member(value: &Value, name: &str) -> Result<String, RpcError> {
    value
        .get(name)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| RpcError::invalid_params(format!("the argument `{name}` is missing")))
}

/// Reads an optional string member of an object.
pub fn optional_string(value: &Value, name: &str) -> Result<Option<String>, RpcError> {
    match value.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(RpcError::invalid_params(format!(
            "the argument `{name}` must be a string"
        ))),
    }
}

/// Reads an optional number member of an object.
pub fn optional_usize(value: &Value, name: &str) -> Result<Option<usize>, RpcError> {
    match value.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => number
            .as_u64()
            .map(|value| Some(value as usize))
            .ok_or_else(|| {
                RpcError::invalid_params(format!("the argument `{name}` must be a whole number"))
            }),
        Some(_) => Err(RpcError::invalid_params(format!(
            "the argument `{name}` must be a whole number"
        ))),
    }
}

/// Reads an optional boolean member of an object.
pub fn optional_bool(value: &Value, name: &str) -> Result<Option<bool>, RpcError> {
    match value.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(flag)) => Ok(Some(*flag)),
        Some(_) => Err(RpcError::invalid_params(format!(
            "the argument `{name}` must be true or false"
        ))),
    }
}

/// Reads the arguments of a tool.
pub fn arguments_of(params: &Value) -> Value {
    match params.get("arguments") {
        Some(value @ Value::Object(_)) => value.clone(),
        _ => json!({}),
    }
}

/// Reads the name of the tool of a call.
pub fn tool_name_of(params: &Value) -> Result<String, RpcError> {
    params
        .get("name")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| RpcError::invalid_params("the `name` of the tool is missing"))
}

/// A resource of the vault is addressed by a URI of this scheme.
pub const RESOURCE_SCHEME: &str = "mcpad://vault/";

/// Builds the URI of a file of the vault.
pub fn resource_uri(relative: &str) -> String {
    format!("{RESOURCE_SCHEME}{relative}")
}

/// Reads the path of a file out of a resource URI.
pub fn path_of_resource(uri: &str) -> Result<String, RpcError> {
    let path = uri.strip_prefix(RESOURCE_SCHEME).ok_or_else(|| {
        RpcError::invalid_params(format!("`{uri}` is not an address of a file of the vault"))
    })?;
    if path.is_empty() {
        return Err(RpcError::invalid_params("the address names no file"));
    }
    Ok(path.to_string())
}

/// The kind of content of a file.
pub fn mime_type_of(relative: &str) -> &'static str {
    match relative.rsplit('.').next() {
        Some("md") | Some("markdown") => "text/markdown",
        Some("txt") => "text/plain",
        Some("json") => "application/json",
        Some("canvas") => "application/json",
        Some("html") | Some("htm") => "text/html",
        Some("css") => "text/css",
        Some("js") | Some("ts") => "text/javascript",
        Some("csv") => "text/csv",
        _ => "text/plain",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_known_revision_is_answered_with_the_same_one() {
        assert_eq!(negotiate(Some("2025-03-26")), "2025-03-26");
        assert_eq!(negotiate(Some("2024-11-05")), "2024-11-05");
    }

    #[test]
    fn an_unknown_revision_is_answered_with_the_newest_one() {
        assert_eq!(negotiate(Some("2030-01-01")), LATEST_PROTOCOL_VERSION);
        assert_eq!(negotiate(None), LATEST_PROTOCOL_VERSION);
    }

    #[test]
    fn the_structured_output_arrived_with_the_revision_of_june_2025() {
        assert!(!supports_structured_output("2025-03-26"));
        assert!(supports_structured_output("2025-06-18"));
        assert!(supports_structured_output("2025-11-25"));
    }

    #[test]
    fn a_request_is_read_out_of_a_message() {
        let message = json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/list",
            "params": { "cursor": null }
        });

        let request = read_request(&message).unwrap();
        assert_eq!(request.method, "tools/list");
        assert_eq!(request.id, Some(json!(7)));
    }

    #[test]
    fn a_notification_has_no_id() {
        let message = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        let request = read_request(&message).unwrap();
        assert!(request.id.is_none());
    }

    #[test]
    fn a_message_without_the_version_is_refused() {
        let message = json!({ "id": 1, "method": "tools/list" });
        let error = read_request(&message).unwrap_err();
        assert_eq!(error.code, INVALID_REQUEST);
    }

    #[test]
    fn a_message_without_a_method_is_refused() {
        let message = json!({ "jsonrpc": "2.0", "id": 1 });
        let error = read_request(&message).unwrap_err();
        assert_eq!(error.code, INVALID_REQUEST);
    }

    #[test]
    fn the_arguments_of_a_call_are_read() {
        let params = json!({ "name": "read_file", "arguments": { "path": "a.md" } });
        assert_eq!(tool_name_of(&params).unwrap(), "read_file");
        assert_eq!(
            string_member(&arguments_of(&params), "path").unwrap(),
            "a.md"
        );
    }

    #[test]
    fn a_missing_argument_is_reported() {
        let error = string_member(&json!({}), "path").unwrap_err();
        assert_eq!(error.code, INVALID_PARAMS);
    }

    #[test]
    fn the_address_of_a_file_carries_its_path() {
        let uri = resource_uri("notes/a.md");
        assert_eq!(uri, "mcpad://vault/notes/a.md");
        assert_eq!(path_of_resource(&uri).unwrap(), "notes/a.md");
        assert!(path_of_resource("https://example.com").is_err());
    }

    #[test]
    fn the_kind_of_a_file_is_guessed_by_its_name() {
        assert_eq!(mime_type_of("notes/a.md"), "text/markdown");
        assert_eq!(mime_type_of("notes/a.txt"), "text/plain");
        assert_eq!(mime_type_of("notes/a.unknown"), "text/plain");
    }
}
