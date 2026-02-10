// Path: src-tauri/src/lib/commands/startup.rs
// Description: Startup readiness command for splashscreen -> main transition

use crate::config::{load_from_disk, resolve_config_path, types::resolve_window_bounds_for_mode};
use crate::obs::logging;
use tauri::{AppHandle, LogicalSize, Manager, Size};

pub fn apply_launch_window_bounds(app: &AppHandle) {
    let config = match resolve_config_path(app).and_then(|path| {
        load_from_disk(&path)
            .map(|result| result.config)
            .map_err(|error| error.to_string())
    }) {
        Ok(config) => config,
        Err(err) => {
            logging::log(
                "warn",
                "startup",
                "launch_bounds_config_failed",
                &format!("Falling back to default launch bounds: {err}"),
            );
            crate::config::PersistedConfig::default()
        }
    };

    let bounds = resolve_window_bounds_for_mode(&config, config.ui_mode);
    for label in ["main", "splashscreen"] {
        let Some(window) = app.get_webview_window(label) else {
            continue;
        };
        let size = Size::Logical(LogicalSize::new(bounds.width as f64, bounds.height as f64));
        if let Err(err) = window.set_size(size) {
            logging::log(
                "warn",
                "startup",
                "launch_bounds_apply_failed",
                &format!("Failed to set {label} window size: {err}"),
            );
        }
    }
}

/// Marks frontend startup as ready and transitions from splash to main window.
/// Idempotent: safe to call multiple times.
#[tauri::command]
pub fn startup_ready(app: AppHandle) -> Result<(), String> {
    if let Some(splash) = app.get_webview_window("splashscreen") {
        if let Err(err) = splash.close() {
            logging::log(
                "warn",
                "startup",
                "close_splash_failed",
                &format!("Failed to close splashscreen window: {err}"),
            );
        }
    }

    let main_window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;

    if let Err(err) = main_window.show() {
        return Err(format!("Failed to show main window: {err}"));
    }

    if let Err(err) = main_window.set_focus() {
        return Err(format!("Failed to focus main window: {err}"));
    }

    Ok(())
}
