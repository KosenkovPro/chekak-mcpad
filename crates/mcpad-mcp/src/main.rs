//! The MCP server of MCPad as a program.

use std::io::{self, BufReader};
use std::process::ExitCode;
use std::sync::Arc;

use mcpad_core::Result;
use mcpad_mcp::cli::{self, Transport};
use mcpad_mcp::{AllowGate, DenyGate, Gate, HttpServer, McpServer, VaultBackend};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mcpad-mcp: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Serves the vault the user asked for.
fn run() -> Result<()> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let options = cli::parse(&arguments)?;

    if options.help {
        print!("{}", cli::usage());
        return Ok(());
    }

    if options.version {
        println!("mcpad-mcp {}", mcpad_mcp::SERVER_VERSION);
        return Ok(());
    }

    let resolved = cli::resolve(&options)?;
    let backend = VaultBackend::open(&resolved.vault)?;

    let gate: Arc<dyn Gate> = if resolved.allow_writes {
        Arc::new(AllowGate)
    } else {
        Arc::new(DenyGate)
    };
    let server = Arc::new(McpServer::new(backend, gate));

    match options.transport {
        Transport::Stdio => {
            eprintln!(
                "mcpad-mcp: the vault {} is served over stdio{}",
                resolved.vault.display(),
                mode_of(resolved.allow_writes)
            );
            let input = BufReader::new(io::stdin());
            let mut output = io::stdout();
            mcpad_mcp::stdio::serve(&server, input, &mut output)
        }
        Transport::Http => {
            let http = HttpServer::bind(resolved.port, server, resolved.token.clone())?;
            eprintln!(
                "mcpad-mcp: the vault {} is served on {}{}",
                resolved.vault.display(),
                http.url()?,
                mode_of(resolved.allow_writes)
            );
            http.serve()
        }
    }
}

/// The mode of the server, for the log.
fn mode_of(allow_writes: bool) -> &'static str {
    if allow_writes {
        ", the changes are made without a confirmation"
    } else {
        " in the read only mode"
    }
}
