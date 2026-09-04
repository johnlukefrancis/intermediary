// Path: src-tauri/src/lib/commands/terminal.rs
// Description: Tauri commands of the integrated terminal: open, raw-body write, resize, ack, close and the Rust-side clipboard read

use crate::commands::wsl_distro::resolve_runtime_wsl_distro;
use crate::config::types::RepoRoot;
use crate::obs::logging;
use crate::terminal::frames::{
    CloseOutcome, CloseReason, TerminalOpenRequest, TerminalOpened, SESSION_HEADER,
};
use crate::terminal::{open_session, read_clipboard_text, TerminalRegistry};
use tauri::ipc::{Channel, InvokeBody, InvokeResponseBody, Request};
use tauri::{AppHandle, State};

/// Spawns pwsh into a pseudoconsole and streams its output on `on_output`.
/// Blocking throughout (registry read, process creation), so it runs off the
/// async runtime.
#[tauri::command]
pub async fn terminal_open(
    app: AppHandle,
    registry: State<'_, TerminalRegistry>,
    request: TerminalOpenRequest,
    on_output: Channel<InvokeResponseBody>,
) -> Result<TerminalOpened, String> {
    let registry = registry.inner().clone();
    // Capture the page identity before this request leaves the IPC handler.
    // A queued blocking worker must not adopt a generation that a later
    // navigation created for a different webview document.
    let page_generation = registry.page_generation()?;
    tauri::async_runtime::spawn_blocking(move || {
        let distro = match &request.repo_root {
            RepoRoot::Wsl { .. } => resolve_runtime_wsl_distro(&app, None),
            RepoRoot::Host { .. } => None,
        };
        open_session(&registry, request, distro, on_output, page_generation)
    })
    .await
    .map_err(|err| task_failed("open", err))?
}

/// Raw-body write: the session id rides in a header, the bytes are the body.
/// An unknown or closing session is an error the frontend treats as benign
/// after the exit frame (I8).
#[tauri::command]
pub async fn terminal_write(
    request: Request<'_>,
    registry: State<'_, TerminalRegistry>,
) -> Result<(), String> {
    let session_id = session_id_from_headers(&request)?;
    let bytes = match request.body() {
        InvokeBody::Raw(bytes) => bytes.clone(),
        InvokeBody::Json(_) => {
            return Err("terminal_write expects a raw byte body, not JSON".to_string())
        }
    };
    let session = registry.running(&session_id)?;
    tauri::async_runtime::spawn_blocking(move || session.write(&bytes))
        .await
        .map_err(|err| task_failed("write", err))?
}

#[tauri::command]
pub async fn terminal_resize(
    registry: State<'_, TerminalRegistry>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let session = registry.running(&session_id)?;
    tauri::async_runtime::spawn_blocking(move || session.resize(cols, rows))
        .await
        .map_err(|err| task_failed("resize", err))?
}

/// Lock-only: advances the cumulative consumed watermark.
#[tauri::command]
pub fn terminal_ack(
    registry: State<'_, TerminalRegistry>,
    session_id: String,
    consumed_total: u64,
) -> Result<(), String> {
    registry.acknowledge(&session_id, consumed_total)
}

/// Console-first close; capacity remains occupied until every resource joins.
#[tauri::command]
pub async fn terminal_close(
    registry: State<'_, TerminalRegistry>,
    session_id: String,
) -> Result<CloseOutcome, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.close(&session_id, CloseReason::Closed))
        .await
        .map_err(|err| task_failed("close", err))?
}

/// WebView2 cannot read the clipboard without a permission prompt; the paste
/// route reads it here instead.
#[tauri::command]
pub async fn terminal_clipboard_text() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(read_clipboard_text)
        .await
        .map_err(|err| task_failed("clipboard_text", err))?
}

fn session_id_from_headers(request: &Request<'_>) -> Result<String, String> {
    request
        .headers()
        .get(SESSION_HEADER)
        .ok_or_else(|| format!("terminal_write is missing the {SESSION_HEADER} header"))?
        .to_str()
        .map(str::to_string)
        .map_err(|_| format!("The {SESSION_HEADER} header is not valid text"))
}

fn task_failed(command: &str, err: tauri::Error) -> String {
    let message = format!("Terminal {command} task failed: {err}");
    logging::log("error", "terminal", "command_task_failed", &message);
    message
}
