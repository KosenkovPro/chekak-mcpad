//! The HTTP transport of the protocol.
//!
//! The server listens on the loopback interface only, so the vault of the
//! user is never reachable from the network. A client that talks to it has to
//! know the token, when one is set.

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;

use mcpad_core::{Error, Result};
use serde_json::{json, Value};

use crate::backend::Backend;
use crate::server::McpServer;

/// The path the protocol is served on.
pub const MCP_PATH: &str = "/mcp";

/// The path that tells whether the server is alive.
pub const HEALTH_PATH: &str = "/health";

/// The largest request body the server accepts.
const MAX_BODY: usize = 8 * 1024 * 1024;

/// A server that answers the requests of the clients over HTTP.
pub struct HttpServer<B: Backend> {
    listener: TcpListener,
    server: Arc<McpServer<B>>,
    token: Option<String>,
}

impl<B: Backend + 'static> HttpServer<B> {
    /// Binds the server to a port of the loopback interface.
    ///
    /// A port of zero asks the system for a free port.
    pub fn bind(port: u16, server: Arc<McpServer<B>>, token: Option<String>) -> Result<Self> {
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        let listener = TcpListener::bind(address)
            .map_err(|error| Error::message(format!("cannot listen on {address}: {error}")))?;
        Ok(Self {
            listener,
            server,
            token,
        })
    }

    /// The address the server listens on.
    pub fn local_address(&self) -> Result<SocketAddr> {
        self.listener.local_addr().map_err(Error::IoBare)
    }

    /// The address of the protocol as a string.
    pub fn url(&self) -> Result<String> {
        Ok(format!("http://{}{MCP_PATH}", self.local_address()?))
    }

    /// Serves the requests until the program ends.
    pub fn serve(self) -> Result<()> {
        for stream in self.listener.incoming() {
            let stream = stream.map_err(Error::IoBare)?;
            let server = Arc::clone(&self.server);
            let token = self.token.clone();

            std::thread::spawn(move || {
                if let Err(error) = handle_connection(stream, &server, token.as_deref()) {
                    eprintln!("mcpad-mcp: {error}");
                }
            });
        }

        Ok(())
    }
}

/// Answers one connection.
fn handle_connection<B: Backend + 'static>(
    stream: TcpStream,
    server: &McpServer<B>,
    token: Option<&str>,
) -> Result<()> {
    let peer_is_loopback = stream
        .peer_addr()
        .map(|address| address.ip().is_loopback())
        .unwrap_or(false);

    let mut reader = BufReader::new(stream.try_clone().map_err(Error::IoBare)?);
    let mut writer = stream;

    let Some(request) = read_request(&mut reader)? else {
        return Ok(());
    };

    if !peer_is_loopback {
        return write_response(
            &mut writer,
            403,
            "Forbidden",
            "application/json",
            &json!({ "error": "only the loopback interface is allowed" }).to_string(),
            &[],
        );
    }

    if let Some(token) = token {
        if !is_authorized(&request, token) {
            return write_response(
                &mut writer,
                401,
                "Unauthorized",
                "application/json",
                &json!({ "error": "the token is missing or wrong" }).to_string(),
                &[],
            );
        }
    }

    if let Some(version) = request.header("mcp-protocol-version") {
        server.set_protocol_version(version);
    }
    let version = server.protocol_version();
    let extra = [("MCP-Protocol-Version", version.as_str())];

    match (request.method.as_str(), request.path.as_str()) {
        ("POST", MCP_PATH) => {
            let body = crate::protocol::strip_bom(&request.body);
            let message: Value = match serde_json::from_str(body) {
                Ok(message) => message,
                Err(error) => {
                    return write_response(
                        &mut writer,
                        400,
                        "Bad Request",
                        "application/json",
                        &json!({ "error": error.to_string() }).to_string(),
                        &extra,
                    )
                }
            };

            match server.handle(message) {
                Some(answer) => {
                    let body = serde_json::to_string(&answer).unwrap_or_default();
                    write_response(&mut writer, 200, "OK", "application/json", &body, &extra)
                }
                None => {
                    write_response(&mut writer, 202, "Accepted", "application/json", "", &extra)
                }
            }
        }
        ("GET", HEALTH_PATH) => write_response(
            &mut writer,
            200,
            "OK",
            "application/json",
            &json!({
                "status": "ok",
                "server": crate::server::SERVER_NAME,
                "version": crate::server::SERVER_VERSION,
                "protocolVersion": version,
            })
            .to_string(),
            &extra,
        ),
        ("GET", "/") => write_response(
            &mut writer,
            200,
            "OK",
            "text/plain; charset=utf-8",
            "MCPad MCP server. Send the JSON-RPC messages of the Model Context Protocol \
             to /mcp.\n",
            &extra,
        ),
        _ => write_response(
            &mut writer,
            404,
            "Not Found",
            "application/json",
            &json!({ "error": format!("there is nothing at `{}`", request.path) }).to_string(),
            &extra,
        ),
    }
}

/// A request that was read off the wire.
struct HttpRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: String,
}

impl HttpRequest {
    /// The value of a header, by its name in lower case.
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// Reads a request off the wire.
fn read_request(reader: &mut impl BufRead) -> Result<Option<HttpRequest>> {
    let mut line = String::new();
    if reader.read_line(&mut line).map_err(Error::IoBare)? == 0 {
        return Ok(None);
    }

    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();

    let mut headers = Vec::new();
    let mut content_length = 0usize;

    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).map_err(Error::IoBare)? == 0 {
            break;
        }

        let header = header.trim_end();
        if header.is_empty() {
            break;
        }

        if let Some((name, value)) = header.split_once(':') {
            let name = name.trim().to_lowercase();
            let value = value.trim().to_string();
            if name == "content-length" {
                content_length = value.parse().unwrap_or(0);
            }
            headers.push((name, value));
        }
    }

    if content_length > MAX_BODY {
        return Err(Error::message(format!(
            "the request is bigger than {MAX_BODY} bytes"
        )));
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).map_err(Error::IoBare)?;
    }

    Ok(Some(HttpRequest {
        method,
        path,
        headers,
        body: String::from_utf8_lossy(&body).into_owned(),
    }))
}

/// Writes an answer.
fn write_response(
    stream: &mut impl Write,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &str,
    extra: &[(&str, &str)],
) -> Result<()> {
    let mut head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    for (name, value) in extra {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");

    stream.write_all(head.as_bytes()).map_err(Error::IoBare)?;
    stream.write_all(body.as_bytes()).map_err(Error::IoBare)?;
    stream.flush().map_err(Error::IoBare)?;

    Ok(())
}

/// Whether the client may talk to the server.
fn is_authorized(request: &HttpRequest, token: &str) -> bool {
    if let Some(value) = request.header("x-mcpad-token") {
        if value == token {
            return true;
        }
    }

    if let Some(value) = request.header("authorization") {
        if let Some(rest) = value.strip_prefix("Bearer ") {
            if rest.trim() == token {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use std::io::Read as _;
    use std::net::TcpStream;
    use std::sync::Arc;

    use serde_json::Value;

    use super::*;
    use crate::gate::DenyGate;
    use crate::vault::VaultBackend;

    fn server_with_notes() -> (tempfile::TempDir, Arc<McpServer<VaultBackend>>) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("notes")).unwrap();
        std::fs::write(dir.path().join("notes/a.md"), "# A\n").unwrap();
        let backend = VaultBackend::open(dir.path()).unwrap();
        let server = Arc::new(McpServer::new(backend, Arc::new(DenyGate)));
        (dir, server)
    }

    /// Starts a server on a free port and returns its address.
    fn start(server: Arc<McpServer<VaultBackend>>, token: Option<String>) -> SocketAddr {
        let http = HttpServer::bind(0, server, token).unwrap();
        let address = http.local_address().unwrap();
        std::thread::spawn(move || {
            let _ = http.serve();
        });
        address
    }

    /// Sends a request and returns the answer.
    fn request(address: SocketAddr, raw: &str) -> String {
        let mut stream = TcpStream::connect(address).unwrap();
        stream.write_all(raw.as_bytes()).unwrap();
        stream.flush().unwrap();

        let mut answer = String::new();
        stream.read_to_string(&mut answer).unwrap();
        answer
    }

    /// The body of an answer.
    fn body_of(answer: &str) -> String {
        match answer.split_once("\r\n\r\n") {
            Some((_, body)) => body.to_string(),
            None => String::new(),
        }
    }

    /// Sends a message to the protocol and returns the answer.
    fn post(address: SocketAddr, body: &str, headers: &str) -> String {
        let raw = format!(
            "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\n{headers}Content-Length: {}\r\n\
             Connection: close\r\n\r\n{body}",
            body.len()
        );
        request(address, &raw)
    }

    #[test]
    fn a_request_is_answered_with_the_protocol() {
        let (_dir, server) = server_with_notes();
        let address = start(server, None);

        let answer = post(
            address,
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#,
            "Content-Type: application/json\r\n",
        );

        assert!(answer.starts_with("HTTP/1.1 200 OK"));
        assert!(answer.contains("MCP-Protocol-Version:"));

        let body: Value = serde_json::from_str(&body_of(&answer)).unwrap();
        assert_eq!(body["id"], 1);
        assert_eq!(body["result"]["tools"].as_array().unwrap().len(), 7);
    }

    #[test]
    fn a_notification_is_answered_with_nothing() {
        let (_dir, server) = server_with_notes();
        let address = start(server, None);

        let answer = post(
            address,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            "",
        );

        assert!(answer.starts_with("HTTP/1.1 202 Accepted"));
        assert_eq!(body_of(&answer), "");
    }

    #[test]
    fn a_client_without_the_token_is_refused() {
        let (_dir, server) = server_with_notes();
        let address = start(server, Some("secret".to_string()));

        let answer = post(address, r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#, "");

        assert!(answer.starts_with("HTTP/1.1 401 Unauthorized"));
    }

    #[test]
    fn a_client_with_the_token_is_answered() {
        let (_dir, server) = server_with_notes();
        let address = start(server, Some("secret".to_string()));

        let answer = post(
            address,
            r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#,
            "Authorization: Bearer secret\r\n",
        );

        assert!(answer.starts_with("HTTP/1.1 200 OK"));
    }

    #[test]
    fn the_health_of_the_server_can_be_asked() {
        let (_dir, server) = server_with_notes();
        let address = start(server, None);

        let answer = request(
            address,
            "GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        );

        assert!(answer.starts_with("HTTP/1.1 200 OK"));
        let body: Value = serde_json::from_str(&body_of(&answer)).unwrap();
        assert_eq!(body["status"], "ok");
        assert_eq!(body["server"], "mcpad");
    }

    #[test]
    fn an_unknown_path_is_refused() {
        let (_dir, server) = server_with_notes();
        let address = start(server, None);

        let answer = request(
            address,
            "GET /nowhere HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        );

        assert!(answer.starts_with("HTTP/1.1 404 Not Found"));
    }

    #[test]
    fn a_broken_message_is_refused() {
        let (_dir, server) = server_with_notes();
        let address = start(server, None);

        let answer = post(address, "hello", "");

        assert!(answer.starts_with("HTTP/1.1 400 Bad Request"));
    }
}
