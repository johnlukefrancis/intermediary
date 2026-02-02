// Path: src-tauri/src/lib/paths/app_paths.rs
// Description: Application path resolution logic

use crate::paths::wsl_convert::windows_to_wsl_path;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// Resolved application paths
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPaths {
    /// Windows AppData\Local directory for this app
    pub app_local_data_dir: String,
    /// Windows path to staging root
    pub staging_windows_root: String,
    /// WSL equivalent path to staging root
    pub staging_wsl_root: String,
    /// Log directory path
    pub log_dir: String,
    /// Path to drag icon PNG
    pub drag_icon_windows_path: String,
}

impl AppPaths {
    /// Resolve all application paths from the Tauri app handle.
    /// If `output_windows_root` is provided, use it as the staging root.
    pub fn resolve(
        app: &AppHandle,
        output_windows_root: Option<&str>,
    ) -> Result<Self, AppPathsError> {
        let app_local_data = app
            .path()
            .app_local_data_dir()
            .map_err(|_| AppPathsError::NoAppLocalData)?;

        ensure_dir(&app_local_data, "app local data directory")?;
        let app_local_data_str = path_to_string(&app_local_data)?;

        // Use override if provided, otherwise default to app_local_data/staging.
        let staging_windows = match output_windows_root {
            Some(override_path) => {
                let trimmed = override_path.trim();
                validate_windows_drive_path(trimmed)?;
                PathBuf::from(trimmed)
            }
            None => app_local_data.join("staging"),
        };
        ensure_dir(&staging_windows, "staging directory")?;
        let staging_windows_str = path_to_string(&staging_windows)?;

        let staging_wsl_root = windows_to_wsl_path(&staging_windows_str)
            .ok_or(AppPathsError::WslConversionFailed)?;

        let log_dir = resolve_log_dir(&app_local_data)?;
        let log_dir_str = path_to_string(&log_dir)?;

        let drag_icon_path = app_local_data.join("drag_icon.png");
        write_drag_icon_if_missing(&drag_icon_path)?;
        let drag_icon_windows_path = path_to_string(&drag_icon_path)?;

        Ok(Self {
            app_local_data_dir: app_local_data_str,
            staging_windows_root: staging_windows_str,
            staging_wsl_root,
            log_dir: log_dir_str,
            drag_icon_windows_path,
        })
    }
}

/// Errors that can occur during path resolution
#[derive(Debug)]
pub enum AppPathsError {
    NoAppLocalData,
    InvalidPath,
    WslConversionFailed,
    InvalidOutputRoot(String),
    Io {
        context: &'static str,
        source: std::io::Error,
    },
}

impl std::fmt::Display for AppPathsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoAppLocalData => write!(f, "Could not resolve app local data directory"),
            Self::InvalidPath => write!(f, "Path contains invalid UTF-8"),
            Self::WslConversionFailed => write!(f, "Failed to convert Windows path to WSL path"),
            Self::InvalidOutputRoot(reason) => {
                write!(f, "Invalid output folder override: {reason}")
            }
            Self::Io { context, source } => write!(f, "{context}: {source}"),
        }
    }
}

impl std::error::Error for AppPathsError {}

fn ensure_dir(path: &Path, context: &'static str) -> Result<(), AppPathsError> {
    fs::create_dir_all(path).map_err(|source| AppPathsError::Io { context, source })
}

fn path_to_string(path: &Path) -> Result<String, AppPathsError> {
    path.to_str()
        .ok_or(AppPathsError::InvalidPath)
        .map(|value| value.to_string())
}

fn resolve_log_dir(app_local_data: &PathBuf) -> Result<PathBuf, AppPathsError> {
    if let Ok(env_dir) = std::env::var("INTERMEDIARY_LOG_DIR") {
        let path = PathBuf::from(env_dir);
        ensure_dir(&path, "log directory")?;
        return Ok(path);
    }

    let log_dir = app_local_data.join("logs");
    ensure_dir(&log_dir, "log directory")?;
    Ok(log_dir)
}

fn write_drag_icon_if_missing(path: &Path) -> Result<(), AppPathsError> {
    if path.exists() {
        return Ok(());
    }

    const DRAG_ICON_BYTES: &[u8] = include_bytes!("../../../icons/32x32.png");
    fs::write(path, DRAG_ICON_BYTES)
        .map_err(|source| AppPathsError::Io { context: "drag icon", source })
}

/// Validate that path is a Windows drive path (not UNC/wsl$).
/// Valid paths start with a drive letter, colon, and slash/backslash (e.g., C:\Users\...).
fn validate_windows_drive_path(path: &str) -> Result<(), AppPathsError> {
    let trimmed = path.trim();

    if trimmed.is_empty() {
        return Err(AppPathsError::InvalidOutputRoot(
            "path cannot be empty".to_string(),
        ));
    }

    // Reject UNC paths (\\server\share or \\wsl$\...)
    if trimmed.starts_with(r"\\") {
        return Err(AppPathsError::InvalidOutputRoot(
            "UNC paths (\\\\...) are not supported".to_string(),
        ));
    }

    // Must start with drive letter followed by colon and slash/backslash
    let mut chars = trimmed.chars();
    let first = chars.next();
    let second = chars.next();
    let third = chars.next();

    match (first, second, third) {
        (Some(c), Some(':'), Some('\\' | '/')) if c.is_ascii_alphabetic() => Ok(()),
        _ => Err(AppPathsError::InvalidOutputRoot(
            "path must start with a drive root (e.g., C:\\\\Projects)".to_string(),
        )),
    }
}
