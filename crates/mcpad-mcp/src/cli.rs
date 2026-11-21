//! The command line of the MCP server.
//!
//! The program is started by an AI client, therefore the options are few and
//! the defaults are safe: the server reads the vault and never writes to it
//! without the explicit flag that allows changes.

use std::path::PathBuf;

use mcpad_core::{Error, Result};

/// The port the HTTP transport uses when nothing else is asked for.
pub const DEFAULT_PORT: u16 = 17171;

/// The transport the server speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// The messages come from the standard input and go to the standard output.
    Stdio,
    /// The messages come from a local HTTP port.
    Http,
}

/// The options of the program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// The transport to serve.
    pub transport: Transport,
    /// The vault to serve, when it is named.
    pub vault: Option<PathBuf>,
    /// Whether the changes are made without the confirmation of the user.
    pub allow_writes: bool,
    /// The port of the HTTP transport, when it is named.
    pub port: Option<u16>,
    /// The token the clients have to send, when it is named.
    pub token: Option<String>,
    /// Whether the help was asked for.
    pub help: bool,
    /// Whether the version was asked for.
    pub version: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            transport: Transport::Stdio,
            vault: None,
            allow_writes: false,
            port: None,
            token: None,
            help: false,
            version: false,
        }
    }
}

/// The options after the environment and the memory of the application were
/// taken into account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// The vault to serve.
    pub vault: PathBuf,
    /// Whether the changes are made without the confirmation of the user.
    pub allow_writes: bool,
    /// The port of the HTTP transport.
    pub port: u16,
    /// The token the clients have to send.
    pub token: Option<String>,
}

/// Reads the options out of the arguments of the program.
pub fn parse(args: &[String]) -> Result<Options> {
    let mut options = Options::default();
    let mut index = 0;

    while index < args.len() {
        let argument = args[index].as_str();

        match argument {
            "stdio" => options.transport = Transport::Stdio,
            "http" => options.transport = Transport::Http,
            "-v" | "--vault" => {
                index += 1;
                options.vault = Some(PathBuf::from(value_of(args, index, "--vault")?));
            }
            "--allow-writes" => options.allow_writes = true,
            "--port" => {
                index += 1;
                let raw = value_of(args, index, "--port")?;
                options.port = Some(
                    raw.parse()
                        .map_err(|_| Error::Config(format!("`{raw}` is not a port number")))?,
                );
            }
            "--token" => {
                index += 1;
                options.token = Some(value_of(args, index, "--token")?.to_string());
            }
            "-h" | "--help" => options.help = true,
            "-V" | "--version" => options.version = true,
            other => {
                return Err(Error::Config(format!(
                    "unknown argument `{other}`, try `mcpad-mcp --help`"
                )))
            }
        }

        index += 1;
    }

    Ok(options)
}

/// Completes the options with the environment and the vault of the last run.
pub fn resolve(options: &Options) -> Result<Resolved> {
    let vault = match options.vault.clone().or_else(|| {
        std::env::var_os("MCPAD_VAULT")
            .map(PathBuf::from)
            .filter(|path| path.is_dir())
    }) {
        Some(vault) => vault,
        None => match mcpad_app::recent::last_vault() {
            Some(vault) => vault,
            None => {
                return Err(Error::Config(
                    "no vault to serve: name one with `--vault <path>`, set MCPAD_VAULT \
                     or open a vault in MCPad once"
                        .to_string(),
                ))
            }
        },
    };

    let port = match options.port {
        Some(port) => port,
        None => match std::env::var("MCPAD_MCP_PORT") {
            Ok(raw) => raw.trim().parse().map_err(|_| {
                Error::Config(format!("MCPAD_MCP_PORT holds `{raw}`, which is not a port"))
            })?,
            Err(_) => DEFAULT_PORT,
        },
    };

    let token = options
        .token
        .clone()
        .or_else(|| std::env::var("MCPAD_MCP_TOKEN").ok())
        .filter(|token| !token.is_empty());

    let allow_writes = options.allow_writes || allows_writes_in_environment();

    Ok(Resolved {
        vault,
        allow_writes,
        port,
        token,
    })
}

/// Whether the environment allows the changes without the confirmation.
fn allows_writes_in_environment() -> bool {
    match std::env::var("MCPAD_MCP_ALLOW_WRITES") {
        Ok(value) => {
            let value = value.trim();
            value == "1" || value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("yes")
        }
        Err(_) => false,
    }
}

/// The value of an option that needs one.
fn value_of<'a>(args: &'a [String], index: usize, name: &str) -> Result<&'a str> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| Error::Config(format!("the option `{name}` needs a value")))
}

/// The help of the program.
pub fn usage() -> String {
    format!(
        "mcpad-mcp {version} — the MCP server of MCPad\n\n\
         Usage:\n  \
         mcpad-mcp [stdio|http] [options]\n\n\
         Transports:\n  \
         stdio            the messages come from the standard input (default)\n  \
         http             the messages come from a local HTTP port\n\n\
         Options:\n  \
         -v, --vault <path>   the vault to serve; the vault of the last run is used when it is not given\n  \
         --port <port>        the port of the HTTP transport, {DEFAULT_PORT} by default\n  \
         --token <token>      the token the clients have to send in the header `Authorization`\n  \
         --allow-writes       make the changes without the confirmation of the user\n  \
         -h, --help           show this help\n  \
         -V, --version        show the version\n\n\
         Environment:\n  \
         MCPAD_VAULT              the vault to serve\n  \
         MCPAD_MCP_PORT           the port of the HTTP transport\n  \
         MCPAD_MCP_TOKEN          the token the clients have to send\n  \
         MCPAD_MCP_ALLOW_WRITES   set it to 1 to make the changes without the confirmation\n",
        version = crate::SERVER_VERSION,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn the_default_transport_is_stdio() {
        let options = parse(&[]).unwrap();
        assert_eq!(options.transport, Transport::Stdio);
        assert!(!options.allow_writes);
        assert_eq!(options.vault, None);
    }

    #[test]
    fn the_transport_is_read() {
        let options = parse(&args(&["http"])).unwrap();
        assert_eq!(options.transport, Transport::Http);
    }

    #[test]
    fn the_vault_is_read() {
        let options = parse(&args(&["--vault", "C:/notes"])).unwrap();
        assert_eq!(options.vault, Some(PathBuf::from("C:/notes")));

        let options = parse(&args(&["-v", "C:/notes"])).unwrap();
        assert_eq!(options.vault, Some(PathBuf::from("C:/notes")));
    }

    #[test]
    fn the_port_and_the_token_are_read() {
        let options = parse(&args(&["http", "--port", "1234", "--token", "secret"])).unwrap();
        assert_eq!(options.port, Some(1234));
        assert_eq!(options.token.as_deref(), Some("secret"));
    }

    #[test]
    fn the_changes_can_be_allowed() {
        let options = parse(&args(&["--allow-writes"])).unwrap();
        assert!(options.allow_writes);
    }

    #[test]
    fn the_help_and_the_version_are_read() {
        assert!(parse(&args(&["--help"])).unwrap().help);
        assert!(parse(&args(&["-h"])).unwrap().help);
        assert!(parse(&args(&["--version"])).unwrap().version);
        assert!(parse(&args(&["-V"])).unwrap().version);
    }

    #[test]
    fn an_unknown_argument_is_refused() {
        let error = parse(&args(&["--dance"])).unwrap_err();
        assert!(matches!(error, Error::Config(_)));
    }

    #[test]
    fn an_option_without_a_value_is_refused() {
        let error = parse(&args(&["--vault"])).unwrap_err();
        assert!(matches!(error, Error::Config(_)));
    }

    #[test]
    fn a_broken_port_is_refused() {
        let error = parse(&args(&["--port", "many"])).unwrap_err();
        assert!(matches!(error, Error::Config(_)));
    }

    #[test]
    fn the_usage_tells_what_the_program_can_do() {
        let text = usage();
        assert!(text.contains("--vault"));
        assert!(text.contains("--allow-writes"));
        assert!(text.contains("MCPAD_VAULT"));
    }
}
