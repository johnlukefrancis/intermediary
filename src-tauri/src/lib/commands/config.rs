// Path: src-tauri/src/lib/commands/config.rs
// Description: Tauri commands for config persistence

use crate::config::{
    io, resolve_config_path, types::LoadConfigResult, validate_config, PersistedConfig,
};
use crate::obs::logging;
use tauri::AppHandle;

/// Load configuration from disk
///
/// Returns the loaded config along with metadata about whether it was
/// freshly created or migrated. Falls back to defaults on missing file.
#[tauri::command]
pub async fn load_config(app: AppHandle) -> Result<LoadConfigResult, String> {
    let config_path = resolve_config_path(&app).map_err(|err| {
        logging::log("error", "config", "load_failed", &err);
        err
    })?;

    let result = tauri::async_runtime::spawn_blocking(move || io::load_from_disk(&config_path))
        .await
        .map_err(|e| {
            let message = format!("Config load task failed: {e}");
            logging::log("error", "config", "load_failed", &message);
            message
        })?
        .map_err(|e| {
            let message = format!("Config load failed: {e}");
            logging::log("error", "config", "load_failed", &message);
            message
        })?;

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
    let config_path = resolve_config_path(&app).map_err(|err| {
        logging::log("error", "config", "save_failed", &err);
        err
    })?;

    validate_config(&config).map_err(|err| {
        let message = format!("Config validation failed: {err}");
        logging::log("error", "config", "save_failed", &message);
        message
    })?;

    tauri::async_runtime::spawn_blocking(move || io::save_to_disk(&config_path, &config))
        .await
        .map_err(|e| {
            let message = format!("Config save task failed: {e}");
            logging::log("error", "config", "save_failed", &message);
            message
        })?
        .map_err(|e| {
            let message = format!("Config save failed: {e}");
            logging::log("error", "config", "save_failed", &message);
            message
        })
}
