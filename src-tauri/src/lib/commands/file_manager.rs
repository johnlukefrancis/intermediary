// Path: src-tauri/src/lib/commands/file_manager.rs
// Description: Open folders in the host OS file manager

use std::path::Path;
use std::process::Command;

/// Open a folder in the host OS file manager.
///
/// # Arguments
/// * `folder_path` - Host-native absolute path to the folder to open
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
        let path = Path::new(&folder_path);
        let is_windows_unc = cfg!(target_os = "windows") && folder_path.starts_with(r"\\");
        if !is_windows_unc && !path.is_dir() {
            return Err(format!("Folder does not exist: {folder_path}"));
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("explorer")
                .arg(&folder_path)
                .spawn()
                .map_err(|e| format!("Failed to open Explorer: {e}"))?;
            return Ok::<(), String>(());
        }

        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .arg(&folder_path)
                .spawn()
                .map_err(|e| format!("Failed to open Finder: {e}"))?;
            return Ok::<(), String>(());
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            Command::new("xdg-open")
                .arg(&folder_path)
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
