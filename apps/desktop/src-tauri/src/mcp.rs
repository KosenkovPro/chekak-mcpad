//! The MCP server of the application.
//!
//! The server listens on the loopback interface and answers the AI clients
//! that are configured for MCPad. It works in the read only mode: every change
//! an agent asks for waits for the confirmation of the user in the interface.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mcpad_core::Result;
use mcpad_mcp::{
    ChannelGate, Decision, HttpServer, McpServer, SwitchableBackend, VaultBackend, WriteRequest,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// The event that tells the interface that a change waits for the user.
pub const PENDING_EVENT: &str = "mcp://pending-write";

/// The event that tells the interface that a change was answered.
pub const SETTLED_EVENT: &str = "mcp://write-settled";

/// How long the user has to answer before the change is refused.
pub const CONFIRM_TIMEOUT: Duration = Duration::from_secs(120);

/// The state of the MCP server of the application.
pub struct McpState {
    backend: Arc<SwitchableBackend>,
    gate: Arc<ChannelGate>,
    running: Mutex<Option<Running>>,
}

/// A server that listens.
#[derive(Clone)]
struct Running {
    url: String,
    port: u16,
    token: String,
}

/// The status of the server, for the interface.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    /// Whether the server listens.
    pub enabled: bool,
    /// The address of the protocol.
    pub url: Option<String>,
    /// The port of the server.
    pub port: Option<u16>,
    /// The token a client has to send.
    pub token: Option<String>,
    /// The number of the changes that wait for the user.
    pub pending: usize,
}

/// A change that waits for the answer of the user.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingWrite {
    /// The identifier of the request.
    pub id: String,
    /// The tool that asks for the change.
    pub tool: String,
    /// The path of the file, relative to the vault.
    pub path: String,
    /// A short description of the change.
    pub summary: String,
    /// The beginning of the new content.
    pub preview: Option<String>,
}

impl From<&WriteRequest> for PendingWrite {
    fn from(request: &WriteRequest) -> Self {
        Self {
            id: request.id.clone(),
            tool: request.tool.clone(),
            path: request.path.clone(),
            summary: request.summary.clone(),
            preview: request.preview.clone(),
        }
    }
}

impl McpState {
    /// Starts the server on a free port of the loopback interface.
    ///
    /// The token is taken from the environment when it is set, otherwise a new
    /// one is made. A pinned token lets a script of the user talk to the
    /// application without looking into the interface.
    pub fn new() -> Result<Self> {
        let backend = Arc::new(SwitchableBackend::empty());
        let gate = Arc::new(ChannelGate::new(CONFIRM_TIMEOUT));

        let token = std::env::var("MCPAD_MCP_TOKEN")
            .ok()
            .filter(|token| !token.is_empty())
            .unwrap_or_else(make_token);

        let server = Arc::new(McpServer::new(backend.clone(), gate.clone()));
        let http = HttpServer::bind(0, server, Some(token.clone()))?;
        let url = http.url()?;
        let port = http.local_address()?.port();

        std::thread::spawn(move || {
            if let Err(error) = http.serve() {
                eprintln!("mcpad: the MCP server stopped: {error}");
            }
        });

        Ok(Self {
            backend,
            gate,
            running: Mutex::new(Some(Running { url, port, token })),
        })
    }

    /// Starts the server and tells the interface about the changes that wait.
    pub fn start(app: &AppHandle) -> Result<Self> {
        let state = Self::new()?;
        state.watch(app);
        Ok(state)
    }

    /// Tells the interface about every change that waits for the user.
    pub fn watch(&self, app: &AppHandle) {
        let (sender, receiver) = channel();
        self.gate.set_listener(sender);

        let handle = app.clone();
        std::thread::spawn(move || {
            while let Ok(request) = receiver.recv() {
                let _ = handle.emit(PENDING_EVENT, PendingWrite::from(&request));
            }
        });
    }

    /// Puts a vault behind the server.
    pub fn bind_vault(&self, root: &Path) -> Result<()> {
        self.backend.set(Box::new(VaultBackend::open(root)?));
        Ok(())
    }

    /// The status of the server.
    pub fn status(&self) -> McpStatus {
        let running = lock(&self.running);
        match running.as_ref() {
            Some(running) => McpStatus {
                enabled: true,
                url: Some(running.url.clone()),
                port: Some(running.port),
                token: Some(running.token.clone()),
                pending: self.gate.pending_count(),
            },
            None => McpStatus {
                enabled: false,
                url: None,
                port: None,
                token: None,
                pending: self.gate.pending_count(),
            },
        }
    }
    /// The changes that wait for the answer of the user.
    pub fn pending(&self) -> Vec<PendingWrite> {
        self.gate.pending().iter().map(PendingWrite::from).collect()
    }

    /// Answers a change.
    ///
    /// Returns `false` when the request is unknown, for example because the
    /// user was too slow and the request timed out already.
    pub fn resolve(&self, id: &str, allow: bool) -> bool {
        let decision = if allow {
            Decision::Allow
        } else {
            Decision::Deny("the user refused the change".to_string())
        };
        self.gate.resolve(id, decision)
    }
}

/// Builds a token that a client has to send.
///
/// The token guards the vault from the other programs of the machine, it is
/// not a secret of the user: it is written down in the interface and copied
/// into the settings of the client.
fn make_token() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or(0);

    let mut first = RandomState::new().build_hasher();
    first.write_u128(nanos);
    first.write_u32(std::process::id());

    let mut second = RandomState::new().build_hasher();
    second.write_u128(nanos);
    second.write_u64(first.finish());

    format!("{:016x}{:016x}", first.finish(), second.finish())
}

/// Locks the state and ignores a poisoned lock.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    use super::*;

    /// Sends a message to the server and returns the answer.
    fn request(port: u16, token: &str, body: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let raw = format!(
            "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {token}\r\n\
             Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(raw.as_bytes()).unwrap();
        stream.flush().unwrap();

        let mut answer = String::new();
        stream.read_to_string(&mut answer).unwrap();
        answer
    }

    #[test]
    fn the_tokens_are_different() {
        let first = make_token();
        let second = make_token();
        assert_ne!(first, second);
        assert_eq!(first.len(), 32);
        assert!(first.chars().all(|character| character.is_ascii_hexdigit()));
    }

    #[test]
    fn the_server_of_the_application_answers_the_protocol() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "# A\n").unwrap();

        let state = McpState::new().unwrap();
        state.bind_vault(dir.path()).unwrap();
        let status = state.status();
        assert!(status.enabled);
        assert_eq!(status.pending, 0);

        let answer = request(
            status.port.unwrap(),
            status.token.as_deref().unwrap(),
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
        );
        assert!(answer.starts_with("HTTP/1.1 200 OK"));
        assert!(answer.contains("\"read_file\""));
    }

    #[test]
    fn a_change_of_an_agent_waits_for_the_confirmation_of_the_user() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "# A\n").unwrap();

        let state = McpState::new().unwrap();
        state.bind_vault(dir.path()).unwrap();
        let status = state.status();
        let port = status.port.unwrap();
        let token = status.token.clone().unwrap();

        let (sender, receiver) = channel();
        state.gate.set_listener(sender);

        let answer = std::thread::spawn(move || {
            request(
                port,
                &token,
                r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"write_file","arguments":{"path":"b.md","text":"new\n"}}}"#,
            )
        });

        // The change waits for the user, nothing is written yet.
        let pending = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(pending.path, "b.md");
        assert_eq!(pending.tool, "write_file");
        assert!(!dir.path().join("b.md").exists());

        // The user allows the change.
        assert!(state.resolve(&pending.id, true));

        let answer = answer.join().unwrap();
        assert!(answer.contains("is written"));
        assert_eq!(
            std::fs::read_to_string(dir.path().join("b.md")).unwrap(),
            "new\n"
        );
    }

    #[test]
    fn a_refused_change_leaves_the_vault_alone() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "# A\n").unwrap();

        let state = McpState::new().unwrap();
        state.bind_vault(dir.path()).unwrap();
        let status = state.status();
        let port = status.port.unwrap();
        let token = status.token.clone().unwrap();

        let (sender, receiver) = channel();
        state.gate.set_listener(sender);

        let answer = std::thread::spawn(move || {
            request(
                port,
                &token,
                r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"delete_file","arguments":{"path":"a.md"}}}"#,
            )
        });

        let pending = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(pending.tool, "delete_file");
        assert!(state.resolve(&pending.id, false));

        let answer = answer.join().unwrap();
        assert!(answer.contains("isError"));
        assert!(dir.path().join("a.md").exists());
    }
}
