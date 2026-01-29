// Path: src-tauri/src/lib/commands/paths.rs
// Description: get_app_paths command implementation

use crate::paths::app_paths::AppPaths;
use tauri::AppHandle;

/// Returns resolved application paths for staging, logging, etc.
#[tauri::command]
pub fn get_app_paths(app: AppHandle) -> Result<AppPaths, String> {
    AppPaths::resolve(&app).map_err(|e| e.to_string())
}
