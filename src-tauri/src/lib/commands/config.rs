// Path: src-tauri/src/lib/commands/config.rs
// Description: Tauri commands for config persistence

use crate::config::{io, types::LoadConfigResult, validate_config, PersistedConfig};
use tauri::{AppHandle, Manager};

/// Load configuration from disk
///
/// Returns the loaded config along with metadata about whether it was
/// freshly created or migrated. Falls back to defaults on missing file.
#[tauri::command]
pub async fn load_config(app: AppHandle) -> Result<LoadConfigResult, String> {
    let config_path = resolve_config_path(&app)?;

    let result = tauri::async_runtime::spawn_blocking(move || io::load_from_disk(&config_path))
        .await
        .map_err(|e| format!("Config load task failed: {e}"))?
        .map_err(|e| e.to_string())?;

    Ok(LoadConfigResult {
        config: result.config,
        was_created: result.was_created,
        migration_applied: result.migration_applied,
    })
}

/// Save configuration to disk
///
/// Writes atomically (temp file + rename) to prevent corruption.
/// Returns error if validation fails or write fails.
#[tauri::command]
pub async fn save_config(app: AppHandle, config: PersistedConfig) -> Result<(), String> {
    let config_path = resolve_config_path(&app)?;

    validate_config(&config)?;

    tauri::async_runtime::spawn_blocking(move || io::save_to_disk(&config_path, &config))
        .await
        .map_err(|e| format!("Config save task failed: {e}"))?
        .map_err(|e| e.to_string())
}

fn resolve_config_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let app_local_data = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "Could not resolve app local data directory")?;

    // Ensure directory exists
    std::fs::create_dir_all(&app_local_data)
        .map_err(|e| format!("Failed to create config directory: {e}"))?;

    Ok(app_local_data.join("config.json"))
}
