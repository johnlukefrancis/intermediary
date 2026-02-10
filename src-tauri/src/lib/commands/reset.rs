// Path: src-tauri/src/lib/commands/reset.rs
// Description: Tauri command to clear staging artifacts and caches

use crate::obs::logging;
use crate::paths::app_paths::AppPaths;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

const STAGING_SUBDIRS: [&str; 3] = ["files", "bundles", "state"];

/// Reset application state stored on disk.
/// Clears staging artifacts, caches, and per-repo notes without touching repository files.
#[tauri::command]
pub fn reset_app_state(app: AppHandle, output_host_root: Option<String>) -> Result<(), String> {
    let app_local_data = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "Could not resolve app local data directory".to_string())?;

    let default_root = resolve_default_staging_root(&app).map_err(|err| {
        logging::log("error", "config", "reset_failed", &err);
        err
    })?;
    clear_staging_subdirs(&default_root)?;

    if let Some(override_root) = output_host_root {
        let resolved = AppPaths::resolve(&app, Some(override_root.as_str())).map_err(|e| {
            let message = e.to_string();
            logging::log("error", "config", "reset_failed", &message);
            message
        })?;
        let override_path = PathBuf::from(resolved.staging_host_root);
        if override_path != default_root {
            clear_staging_subdirs(&override_path)?;
        }
    }

    // Clear per-repo notes
    let notes_dir = app_local_data.join("notes");
    if notes_dir.exists() {
        fs::remove_dir_all(&notes_dir).map_err(|e| {
            let message = format!("Failed to clear notes: {e}");
            logging::log("error", "config", "reset_failed", &message);
            message
        })?;
    }

    Ok(())
}

fn resolve_default_staging_root(app: &AppHandle) -> Result<PathBuf, String> {
    let app_local_data = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "Could not resolve app local data directory".to_string())?;
    Ok(app_local_data.join("staging"))
}

fn clear_staging_subdirs(root: &Path) -> Result<(), String> {
    for subdir in STAGING_SUBDIRS {
        let target = root.join(subdir);
        if !target.exists() {
            continue;
        }
        fs::remove_dir_all(&target).map_err(|e| {
            let message = format!(
                "Failed to clear staging {subdir} at {}: {e}",
                target.display()
            );
            logging::log("error", "config", "reset_failed", &message);
            message
        })?;
    }

    Ok(())
}
