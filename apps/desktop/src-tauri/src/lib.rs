//! The Tauri shell of MCPad.
//!
//! The shell owns the window and the vault that is currently open. All the
//! behaviour lives in the application service layer (`mcpad-app`), so the
//! commands here stay thin and the logic can be tested without a window.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use mcpad_app::{last_vault, load_theme as load_theme_file, remember_vault};
use mcpad_app::{OpenDocument, VaultFile, VaultService};
use mcpad_core::session::Session;
use mcpad_core::{Error, Result};
use tauri::{AppHandle, Emitter, Manager, State};

mod mcp;

use mcp::{McpState, McpStatus, PendingWrite, SETTLED_EVENT};

/// State of the running application.
#[derive(Default)]
struct AppState {
    service: Mutex<Option<VaultService>>,
    /// The vault the application was started with.
    initial: Option<String>,
}

impl AppState {
    fn service(&self) -> Result<VaultService> {
        let guard = self
            .service
            .lock()
            .map_err(|_| Error::message("the vault lock is poisoned"))?;
        guard
            .clone()
            .ok_or_else(|| Error::message("no vault is open"))
    }
}

/// Lists the files of the open vault.
#[tauri::command]
fn list_files(state: State<'_, AppState>) -> Result<Vec<VaultFile>> {
    state.service()?.list_files()
}

/// Reads a file of the open vault.
#[tauri::command]
fn read_file(path: String, state: State<'_, AppState>) -> Result<OpenDocument> {
    state.service()?.read_file(&path)
}

/// Reads a file of the open vault completely, no matter how big it is.
#[tauri::command]
fn read_file_full(path: String, state: State<'_, AppState>) -> Result<OpenDocument> {
    state.service()?.read_file_full(&path)
}

/// Writes a file of the open vault.
///
/// `complete` tells whether the text is the whole content of the file. A part
/// of a huge file is never written back, because that would cut the rest of
/// the note away.
#[tauri::command]
fn write_file(
    path: String,
    text: String,
    encoding: Option<String>,
    line_ending: Option<String>,
    complete: Option<bool>,
    state: State<'_, AppState>,
) -> Result<()> {
    state.service()?.write_file(
        &path,
        &text,
        encoding.as_deref(),
        line_ending.as_deref(),
        complete.unwrap_or(true),
    )
}

/// Converts a file to another encoding or line ending, in place.
#[tauri::command]
fn convert_file(
    path: String,
    encoding: Option<String>,
    line_ending: Option<String>,
    state: State<'_, AppState>,
) -> Result<()> {
    state
        .service()?
        .convert_file(&path, encoding.as_deref(), line_ending.as_deref())
}

/// Reads the persisted session of the vault.
#[tauri::command]
fn load_session(state: State<'_, AppState>) -> Result<Session> {
    Ok(state.service()?.load_session())
}

/// Writes the persisted session of the vault.
#[tauri::command]
fn save_session(session: Session, state: State<'_, AppState>) -> Result<()> {
    state.service()?.save_session(&session)
}

/// Stores the unsaved content of a file.
#[tauri::command]
fn save_draft(path: String, text: String, state: State<'_, AppState>) -> Result<()> {
    state.service()?.save_draft(&path, &text)
}

/// Reads the unsaved content of a file.
#[tauri::command]
fn load_draft(path: String, state: State<'_, AppState>) -> Result<Option<String>> {
    Ok(state.service()?.load_draft(&path))
}

/// Removes the unsaved content of a file.
#[tauri::command]
fn discard_draft(path: String, state: State<'_, AppState>) -> Result<()> {
    state.service()?.discard_draft(&path)
}

/// Names of the built-in themes.
#[tauri::command]
fn list_themes() -> Vec<String> {
    vec![
        "dark".to_string(),
        "light".to_string(),
        "high-contrast".to_string(),
    ]
}

/// Loads a theme by name.
#[tauri::command]
fn load_theme(name: String) -> Result<serde_json::Value> {
    let theme = load_theme_file(&themes_dir(), &name)?;
    serde_json::to_value(theme).map_err(Error::from)
}

/// The vault the application was started with, if there is one.
#[tauri::command]
fn initial_vault(state: State<'_, AppState>) -> Option<String> {
    state.initial.clone()
}

/// The status of the MCP server of the application.
#[tauri::command]
fn mcp_status(state: State<'_, McpState>) -> McpStatus {
    state.status()
}

/// The changes of the AI clients that wait for the answer of the user.
#[tauri::command]
fn mcp_pending_writes(state: State<'_, McpState>) -> Vec<PendingWrite> {
    state.pending()
}

/// Answers a change of an AI client.
#[tauri::command]
fn mcp_resolve_write(id: String, allow: bool, app: AppHandle, state: State<'_, McpState>) -> bool {
    let answered = state.resolve(&id, allow);
    if answered {
        let _ = app.emit(SETTLED_EVENT, id);
    }
    answered
}

/// The directory with the built-in themes, bundled with the application.
fn themes_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("theme")
        .join("themes")
}

/// Opens a vault and makes it the working directory of the application.
#[tauri::command]
fn open_vault(root: String, state: State<'_, AppState>, mcp: State<'_, McpState>) -> Result<()> {
    let service = VaultService::open(&root)?;
    let mut guard = state
        .service
        .lock()
        .map_err(|_| Error::message("the vault lock is poisoned"))?;
    *guard = Some(service);
    // The vault is remembered, so the next start reopens it without a dialog.
    let _ = remember_vault(Path::new(&root));
    // The MCP server follows the vault of the application.
    mcp.bind_vault(Path::new(&root))?;
    Ok(())
}

/// Decides which vault the application starts with.
///
/// The path from the command line wins, then the vault of the previous run.
fn resolve_vault(argument: Option<String>) -> Option<String> {
    match argument {
        Some(root) => Some(root),
        None => last_vault().map(|path| path.to_string_lossy().into_owned()),
    }
}

/// Starts the desktop shell.
///
/// The first argument is the vault to open, exactly like the planned `ckm .`
/// command of the command line interface.
pub fn run() {
    let argument = std::env::args().nth(1).filter(|arg| !arg.starts_with('-'));
    let initial = resolve_vault(argument);

    tauri::Builder::default()
        .setup(move |app| {
            // The vault is opened before the window appears, so the interface
            // starts with a ready tree instead of an empty screen.
            let service = initial
                .as_ref()
                .and_then(|root| VaultService::open(root).ok());
            if service.is_some() {
                if let Some(root) = initial.as_ref() {
                    let _ = remember_vault(Path::new(root));
                }
            }
            app.manage(AppState {
                service: Mutex::new(service),
                initial: initial.clone(),
            });

            // The MCP server listens on the loopback interface from the start
            // and follows the vault of the application.
            let mcp = McpState::start(app.handle())?;
            if let Some(root) = initial.as_ref() {
                let _ = mcp.bind_vault(Path::new(root));
            }
            app.manage(mcp);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_files,
            read_file,
            read_file_full,
            write_file,
            convert_file,
            load_session,
            save_session,
            save_draft,
            load_draft,
            discard_draft,
            list_themes,
            load_theme,
            initial_vault,
            open_vault,
            mcp_status,
            mcp_pending_writes,
            mcp_resolve_write
        ])
        .run(tauri::generate_context!())
        .expect("failed to start the MCPad shell");
}
