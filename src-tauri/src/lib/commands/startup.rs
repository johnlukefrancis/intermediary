// Path: src-tauri/src/lib/commands/startup.rs
// Description: Startup readiness command for splashscreen -> main transition

use crate::config::{
    load_from_disk, resolve_config_path,
    types::{resolve_window_bounds_for_mode, UiWindowBounds},
};
use crate::obs::logging;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, LogicalSize, Manager, Monitor, PhysicalPosition, Position, Size};

pub struct StartupWindowState {
    transition_completed: AtomicBool,
}

impl Default for StartupWindowState {
    fn default() -> Self {
        Self {
            transition_completed: AtomicBool::new(false),
        }
    }
}

fn logical_to_physical_dimension(value: u32, scale_factor: f64) -> Option<i32> {
    let scaled = (f64::from(value) * scale_factor).round();
    if !scaled.is_finite() {
        return None;
    }
    let clamped = scaled.max(1.0).min(f64::from(i32::MAX));
    Some(clamped as i32)
}

fn resolve_launch_monitor(app: &AppHandle) -> Option<Monitor> {
    if let Some(main_window) = app.get_webview_window("main") {
        match main_window.current_monitor() {
            Ok(Some(monitor)) => return Some(monitor),
            Ok(None) => {}
            Err(err) => {
                logging::log(
                    "warn",
                    "startup",
                    "launch_monitor_current_failed",
                    &format!("Failed to resolve current monitor from main window: {err}"),
                );
            }
        }
    }

    match app.primary_monitor() {
        Ok(monitor) => monitor,
        Err(err) => {
            logging::log(
                "warn",
                "startup",
                "launch_monitor_primary_failed",
                &format!("Failed to resolve primary monitor: {err}"),
            );
            None
        }
    }
}

fn resolve_launch_center_position(
    app: &AppHandle,
    bounds: UiWindowBounds,
) -> Option<PhysicalPosition<i32>> {
    let monitor = resolve_launch_monitor(app)?;
    let work_area = monitor.work_area();
    let scale_factor = monitor.scale_factor();

    let area_x = work_area.position.x;
    let area_y = work_area.position.y;
    let area_width = i32::try_from(work_area.size.width).ok()?;
    let area_height = i32::try_from(work_area.size.height).ok()?;
    let bounds_width = logical_to_physical_dimension(bounds.width, scale_factor)?;
    let bounds_height = logical_to_physical_dimension(bounds.height, scale_factor)?;

    let max_x = area_x.saturating_add(area_width.saturating_sub(bounds_width).max(0));
    let max_y = area_y.saturating_add(area_height.saturating_sub(bounds_height).max(0));
    let centered_x = area_x.saturating_add(area_width.saturating_sub(bounds_width) / 2);
    let centered_y = area_y.saturating_add(area_height.saturating_sub(bounds_height) / 2);

    Some(PhysicalPosition::new(
        centered_x.clamp(area_x, max_x),
        centered_y.clamp(area_y, max_y),
    ))
}

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
    let position = resolve_launch_center_position(app, bounds);
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

        if let Some(position) = position {
            if let Err(err) = window.set_position(Position::Physical(position)) {
                logging::log(
                    "warn",
                    "startup",
                    "launch_position_apply_failed",
                    &format!("Failed to set {label} window position: {err}"),
                );
            }
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

fn retire_splashscreen(app: &AppHandle) {
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
    let startup_state = app.state::<StartupWindowState>();
    if startup_state.transition_completed.load(Ordering::SeqCst) {
        return ensure_main_window_ready(&app);
    }

    ensure_main_window_ready(&app)?;
    retire_splashscreen(&app);
    startup_state
        .transition_completed
        .store(true, Ordering::SeqCst);

    ensure_main_window_ready(&app)
}
