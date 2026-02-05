// Path: src-tauri/src/lib/obs/logging.rs
// Description: File-based logger writing to run_latest.txt

use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Resolve the log directory, preferring INTERMEDIARY_LOG_DIR env var
pub fn resolve_log_dir(app: &AppHandle) -> Option<PathBuf> {
    // Check env var first (set by VS Code task for WSL logging)
    if let Ok(env_dir) = std::env::var("INTERMEDIARY_LOG_DIR") {
        let path = PathBuf::from(&env_dir);
        if fs::create_dir_all(&path).is_ok() {
            return Some(path);
        }
    }

    // Fall back to app local data logs directory
    app.path().app_local_data_dir().ok().map(|p| p.join("logs"))
}

/// Initialize the logger with the given log directory
pub fn init(log_dir: &Path) {
    if LOG_PATH.get().is_some() {
        return; // Already initialized
    }

    // Ensure directory exists
    if fs::create_dir_all(log_dir).is_err() {
        eprintln!("Failed to create log directory: {}", log_dir.display());
        return;
    }

    let log_file = log_dir.join("run_latest.txt");

    // Truncate/create the file at startup
    if let Err(e) = fs::write(&log_file, "") {
        eprintln!("Failed to create log file: {e}");
        return;
    }

    let _ = LOG_PATH.set(log_file);
}

/// Write a log entry
///
/// Format: `[2024-01-15 14:32:01] LEVEL [scope] event details`
pub fn log(level: &str, scope: &str, event: &str, details: &str) {
    let Some(log_path) = LOG_PATH.get() else {
        return;
    };

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let level_upper = level.to_uppercase();
    let line = format!("[{timestamp}] {level_upper} [{scope}] {event} {details}\n");

    let result = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .and_then(|mut f| f.write_all(line.as_bytes()));

    if let Err(e) = result {
        eprintln!("Failed to write log: {e}");
    }
}
