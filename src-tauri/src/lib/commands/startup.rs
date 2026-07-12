// Path: src-tauri/src/lib/commands/startup.rs
// Description: Startup readiness command for splashscreen -> main transition

use super::startup_window_bounds;
use crate::obs::logging;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StartupPhase {
    AwaitingRuntime,
    AwaitingFrontend,
    Complete,
}

pub struct StartupWindowState {
    phase: Mutex<StartupPhase>,
}

impl Default for StartupWindowState {
    fn default() -> Self {
        Self {
            phase: Mutex::new(StartupPhase::AwaitingRuntime),
        }
    }
}

pub fn apply_launch_window_bounds(app: &AppHandle) {
    let Some(startup_state) = app.try_state::<StartupWindowState>() else {
        logging::log(
            "error",
            "startup",
            "launch_state_missing",
            "Startup window state was unavailable at runtime-ready transition",
        );
        return;
    };
    let Ok(mut phase) = startup_state.phase.lock() else {
        logging::log(
            "error",
            "startup",
            "launch_state_poisoned",
            "Startup phase lock was poisoned",
        );
        return;
    };
    if *phase != StartupPhase::AwaitingRuntime {
        return;
    }
    logging::log(
        "info",
        "startup",
        "launch_window_ready_begin",
        "Applying launch window state after Tauri runtime readiness",
    );

    startup_window_bounds::apply(app);
    if let Some(main_window) = app.get_webview_window("main") {
        if let Err(err) = main_window.show() {
            logging::log(
                "warn",
                "startup",
                "show_main_boot_failed",
                &format!("Failed to activate main startup WebView: {err}"),
            );
        }
    }

    if let Some(splash_window) = app.get_webview_window("splashscreen") {
        if let Err(err) = splash_window.show() {
            logging::log(
                "warn",
                "startup",
                "show_splash_failed",
                &format!("Failed to show splashscreen window: {err}"),
            );
        }
    }
    *phase = StartupPhase::AwaitingFrontend;
    logging::log(
        "info",
        "startup",
        "launch_window_ready_complete",
        "Launch window state applied",
    );
}

fn ensure_main_window_ready(app: &AppHandle) -> Result<(), String> {
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

pub fn retire_splashscreen(app: &AppHandle) {
    let Some(splash_window) = app.get_webview_window("splashscreen") else {
        return;
    };

    if let Err(err) = splash_window.hide() {
        logging::log(
            "warn",
            "startup",
            "hide_splash_failed",
            &format!("Failed to hide splashscreen window: {err}"),
        );
    }

    if let Err(err) = splash_window.destroy() {
        logging::log(
            "warn",
            "startup",
            "destroy_splash_failed",
            &format!("Failed to destroy splashscreen window: {err}"),
        );

        if let Err(close_err) = splash_window.close() {
            logging::log(
                "warn",
                "startup",
                "close_splash_fallback_failed",
                &format!("Fallback close for splashscreen window failed: {close_err}"),
            );
        }
    }
}

/// Marks frontend startup as ready and transitions from splash to main window.
/// Idempotent: safe to call multiple times.
#[tauri::command]
pub fn startup_ready(app: AppHandle) -> Result<(), String> {
    let startup_state = app.try_state::<StartupWindowState>().ok_or_else(|| {
        logging::log(
            "error",
            "startup",
            "ready_state_missing",
            "Frontend readiness arrived without registered startup state",
        );
        "Startup window state is unavailable".to_string()
    })?;
    let mut phase = startup_state.phase.lock().map_err(|_| {
        logging::log(
            "error",
            "startup",
            "ready_state_poisoned",
            "Startup phase lock was poisoned",
        );
        "Startup window state is unavailable".to_string()
    })?;
    if *phase == StartupPhase::Complete {
        return ensure_main_window_ready(&app);
    }

    ensure_main_window_ready(&app)?;
    retire_splashscreen(&app);
    *phase = StartupPhase::Complete;
    logging::log(
        "info",
        "startup",
        "transition_complete",
        "Main window revealed and splashscreen retired",
    );

    ensure_main_window_ready(&app)
}

#[cfg(test)]
mod tests {
    use super::{StartupPhase, StartupWindowState};

    #[test]
    fn startup_phase_begins_awaiting_runtime() {
        let state = StartupWindowState::default();
        assert_eq!(
            *state.phase.lock().expect("test lock"),
            StartupPhase::AwaitingRuntime
        );
    }
}
