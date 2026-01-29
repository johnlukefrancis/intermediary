// Path: src-tauri/src/lib/paths/app_paths.rs
// Description: Application path resolution logic

use crate::paths::wsl_convert::windows_to_wsl_path;
use serde::Serialize;
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
    /// Path to drag icon PNG (None until we embed one)
    pub drag_icon_windows_path: Option<String>,
}

impl AppPaths {
    /// Resolve all application paths from the Tauri app handle
    pub fn resolve(app: &AppHandle) -> Result<Self, AppPathsError> {
        let app_local_data = app
            .path()
            .app_local_data_dir()
            .map_err(|_| AppPathsError::NoAppLocalData)?;

        let app_local_data_str = app_local_data
            .to_str()
            .ok_or(AppPathsError::InvalidPath)?
            .to_string();

        let staging_windows = app_local_data.join("staging");
        let staging_windows_str = staging_windows
            .to_str()
            .ok_or(AppPathsError::InvalidPath)?
            .to_string();

        let staging_wsl_root = windows_to_wsl_path(&staging_windows_str)
            .ok_or(AppPathsError::WslConversionFailed)?;

        let log_dir = app_local_data.join("logs");
        let log_dir_str = log_dir
            .to_str()
            .ok_or(AppPathsError::InvalidPath)?
            .to_string();

        Ok(Self {
            app_local_data_dir: app_local_data_str,
            staging_windows_root: staging_windows_str,
            staging_wsl_root,
            log_dir: log_dir_str,
            drag_icon_windows_path: None,
        })
    }
}

/// Errors that can occur during path resolution
#[derive(Debug)]
pub enum AppPathsError {
    NoAppLocalData,
    InvalidPath,
    WslConversionFailed,
}

impl std::fmt::Display for AppPathsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoAppLocalData => write!(f, "Could not resolve app local data directory"),
            Self::InvalidPath => write!(f, "Path contains invalid UTF-8"),
            Self::WslConversionFailed => write!(f, "Failed to convert Windows path to WSL path"),
        }
    }
}

impl std::error::Error for AppPathsError {}
