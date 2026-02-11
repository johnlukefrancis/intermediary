// Path: src-tauri/src/lib/commands/paths.rs
// Description: get_app_paths command implementation and path conversion utilities

use crate::agent::AgentWebSocketAuthState;
use crate::config::types::RepoRoot;
use crate::paths::app_paths::AppPaths;
use crate::paths::repo_root_resolver::{resolve_repo_root_from_input, RepoRootKind};
use crate::paths::wsl_convert::{run_wslpath, windows_to_wsl_path, wsl_to_windows_path};
use tauri::{AppHandle, State};

use super::wsl_distro::resolve_runtime_wsl_distro;

/// Returns resolved application paths for staging, logging, etc.
/// If `output_host_root` is provided, uses it as the staging root.
#[tauri::command]
pub fn get_app_paths(
    app: AppHandle,
    auth_state: State<'_, AgentWebSocketAuthState>,
    output_host_root: Option<String>,
) -> Result<AppPaths, String> {
    let mut paths =
        AppPaths::resolve(&app, output_host_root.as_deref()).map_err(|e| e.to_string())?;
    paths.agent_ws_token = auth_state.host_ws_token().to_string();
    Ok(paths)
}

/// Convert a Windows path to WSL path format.
/// Used by frontend when adding repos via directory picker.
#[tauri::command]
pub fn convert_windows_to_wsl(windows_path: String) -> Result<String, String> {
    if !cfg!(target_os = "windows") {
        return Err(wsl_conversion_unsupported_error());
    }
    windows_to_wsl_path(&windows_path)
        .ok_or_else(|| format!("Invalid Windows path format: {}", windows_path))
}

/// Resolve a user-selected path into a path-native repo root.
/// Critical case: \\wsl$\...\mnt\<drive>\... resolves to a Windows root.
#[tauri::command]
pub fn resolve_repo_root(input_path: String) -> Result<RepoRoot, String> {
    let resolved = resolve_repo_root_from_input(&input_path)
        .ok_or_else(|| format!("Invalid repo root path: {input_path}"))?;

    match resolved.kind {
        RepoRootKind::Wsl => Ok(RepoRoot::Wsl {
            path: resolved.path,
        }),
        RepoRootKind::Host => Ok(RepoRoot::Host {
            path: resolved.path,
        }),
    }
}

/// Convert a WSL path to Windows path format.
/// Handles both /mnt/X/... paths (fast pure conversion) and native WSL paths (via wslpath).
#[tauri::command]
pub async fn convert_wsl_to_windows(
    app: AppHandle,
    wsl_path: String,
    distro_override: Option<String>,
) -> Result<String, String> {
    if !cfg!(target_os = "windows") {
        return Err(wsl_conversion_unsupported_error());
    }

    // Fast path: /mnt/X/... uses pure Rust conversion
    if let Some(win_path) = wsl_to_windows_path(&wsl_path) {
        return Ok(win_path);
    }

    // Native Linux path: call wslpath via subprocess
    let distro_override = resolve_runtime_wsl_distro(&app, distro_override.as_deref());
    tauri::async_runtime::spawn_blocking(move || {
        run_wslpath(&wsl_path, distro_override.as_deref()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}

fn wsl_conversion_unsupported_error() -> String {
    "WSL path conversion is only available on Windows hosts".to_string()
}
