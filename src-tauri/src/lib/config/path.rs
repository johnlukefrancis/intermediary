// Path: src-tauri/src/lib/config/path.rs
// Description: Resolve persisted config file location for app commands and setup

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub fn resolve_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_local_data = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "Could not resolve app local data directory")?;

    std::fs::create_dir_all(&app_local_data)
        .map_err(|e| format!("Failed to create config directory: {e}"))?;

    Ok(app_local_data.join("config.json"))
}
