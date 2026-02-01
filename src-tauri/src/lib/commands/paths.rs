// Path: src-tauri/src/lib/commands/paths.rs
// Description: get_app_paths command implementation and path conversion utilities

use crate::paths::app_paths::AppPaths;
use crate::paths::wsl_convert::windows_to_wsl_path;
use tauri::AppHandle;

/// Returns resolved application paths for staging, logging, etc.
#[tauri::command]
pub fn get_app_paths(app: AppHandle) -> Result<AppPaths, String> {
    AppPaths::resolve(&app).map_err(|e| e.to_string())
}

/// Convert a Windows path to WSL path format.
/// Used by frontend when adding repos via directory picker.
#[tauri::command]
pub fn convert_windows_to_wsl(windows_path: String) -> Result<String, String> {
    windows_to_wsl_path(&windows_path)
        .ok_or_else(|| format!("Invalid Windows path format: {}", windows_path))
}
