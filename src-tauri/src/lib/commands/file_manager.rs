// Path: src-tauri/src/lib/commands/file_manager.rs
// Description: Open folders in the host OS file manager

#[cfg(target_os = "windows")]
use crate::paths::wsl_convert::{run_wslpath, wsl_to_windows_path};
use std::path::Path;
use std::process::Command;

/// Open a folder in the host OS file manager.
///
/// # Arguments
/// * `folder_path` - Absolute folder path to open.
///   On Windows, absolute WSL paths are resolved to host-visible paths.
///
/// # Errors
/// Returns an error if the path is empty or the platform launcher fails.
#[tauri::command]
pub async fn open_in_file_manager(folder_path: String) -> Result<(), String> {
    let folder_path = folder_path.trim().to_string();
    if folder_path.is_empty() {
        return Err("Folder path cannot be empty".to_string());
    }

    tauri::async_runtime::spawn_blocking(move || {
        let host_path = resolve_host_path(&folder_path)?;
        let path = Path::new(&host_path);
        let is_windows_unc = cfg!(target_os = "windows") && host_path.starts_with(r"\\");
        if !is_windows_unc && !path.is_dir() {
            return Err(format!("Folder does not exist: {host_path}"));
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("explorer")
                .arg(&host_path)
                .spawn()
                .map_err(|e| format!("Failed to open Explorer: {e}"))?;
            return Ok::<(), String>(());
        }

        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .arg(&host_path)
                .spawn()
                .map_err(|e| format!("Failed to open Finder: {e}"))?;
            return Ok::<(), String>(());
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            Command::new("xdg-open")
                .arg(&host_path)
                .spawn()
                .map_err(|e| format!("Failed to open file manager: {e}"))?;
            return Ok::<(), String>(());
        }

        #[allow(unreachable_code)]
        Err("open_in_file_manager is not supported on this platform".to_string())
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}

#[cfg(target_os = "windows")]
pub(crate) fn resolve_host_path(folder_path: &str) -> Result<String, String> {
    if !folder_path.starts_with('/') {
        return Ok(folder_path.to_string());
    }

    if let Some(windows_path) = wsl_to_windows_path(folder_path) {
        return Ok(windows_path);
    }

    run_wslpath(folder_path).map_err(|error| {
        format!("Failed to resolve WSL folder path '{folder_path}' to a Windows path: {error}")
    })
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn resolve_host_path(folder_path: &str) -> Result<String, String> {
    Ok(folder_path.to_string())
}
