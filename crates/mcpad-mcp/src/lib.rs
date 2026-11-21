//! The MCP server of MCPad.
//!
//! MCPad speaks the Model Context Protocol, so an AI agent can look into the
//! vault of the user and, with the confirmation of the user, change it. The
//! server works over two transports: the standard input and output, which is
//! what most clients start, and a local HTTP port on the loopback interface.
//!
//! The server works in the read only mode by default. A change is only made
//! when the user agreed to it, either in the interface of the application or,
//! for a server that was started by a script of the user, with the explicit
//! flag that allows changes.

pub mod backend;
pub mod cli;
pub mod gate;
pub mod http;
pub mod protocol;
pub mod server;
pub mod stdio;
pub mod switchable;
pub mod tools;
pub mod vault;

pub use backend::{Backend, DEFAULT_READ_LINES};
pub use gate::{AllowGate, ChannelGate, Decision, DenyGate, Gate, WriteRequest};
pub use http::HttpServer;
pub use server::{McpServer, SERVER_NAME, SERVER_VERSION};
pub use switchable::SwitchableBackend;
pub use vault::VaultBackend;
