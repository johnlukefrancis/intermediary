// Path: src-tauri/src/lib/commands/startup.rs
// Description: Startup readiness command for splashscreen -> main transition

use crate::obs::logging;
use tauri::{AppHandle, Manager};

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
