// Path: src-tauri/src/lib/commands/paths.rs
// Description: get_app_paths command implementation and path conversion utilities

use crate::paths::app_paths::AppPaths;
use crate::paths::wsl_convert::{run_wslpath, windows_to_wsl_path, wsl_to_windows_path};
use tauri::AppHandle;

/// Returns resolved application paths for staging, logging, etc.
/// If `output_windows_root` is provided, uses it as the staging root.
#[tauri::command]
pub fn get_app_paths(
    app: AppHandle,
    output_windows_root: Option<String>,
) -> Result<AppPaths, String> {
    AppPaths::resolve(&app, output_windows_root.as_deref()).map_err(|e| e.to_string())
}

/// Convert a Windows path to WSL path format.
/// Used by frontend when adding repos via directory picker.
#[tauri::command]
pub fn convert_windows_to_wsl(windows_path: String) -> Result<String, String> {
    windows_to_wsl_path(&windows_path)
        .ok_or_else(|| format!("Invalid Windows path format: {}", windows_path))
}

/// Convert a WSL path to Windows path format.
/// Handles both /mnt/X/... paths (fast pure conversion) and native WSL paths (via wslpath).
#[tauri::command]
pub async fn convert_wsl_to_windows(wsl_path: String) -> Result<String, String> {
    // Fast path: /mnt/X/... uses pure Rust conversion
    if let Some(win_path) = wsl_to_windows_path(&wsl_path) {
        return Ok(win_path);
    }

    // Native Linux path: call wslpath via subprocess
    tauri::async_runtime::spawn_blocking(move || {
        run_wslpath(&wsl_path).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}
