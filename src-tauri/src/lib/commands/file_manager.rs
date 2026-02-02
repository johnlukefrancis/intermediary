// Path: src-tauri/src/lib/commands/file_manager.rs
// Description: Open folders in OS file manager (Windows Explorer)

use std::path::Path;
use std::process::Command;

/// Open a folder in the OS file manager (Windows Explorer).
///
/// # Arguments
/// * `folder_path` - Windows path to the folder to open
///
/// # Errors
/// Returns an error if the path is empty or Explorer fails to launch.
#[tauri::command]
pub async fn open_in_file_manager(folder_path: String) -> Result<(), String> {
    let folder_path = folder_path.trim().to_string();
    if folder_path.is_empty() {
        return Err("Folder path cannot be empty".to_string());
    }

    if !cfg!(target_os = "windows") {
        return Err("open_in_file_manager is only supported on Windows".to_string());
    }

    // Use spawn_blocking for the process execution
    tauri::async_runtime::spawn_blocking(move || {
        // Skip is_dir check for UNC paths - Rust's Path APIs don't handle them reliably
        let is_unc = folder_path.starts_with(r"\\");
        if !is_unc {
            let path = Path::new(&folder_path);
            if !path.is_dir() {
                return Err(format!("Folder does not exist: {folder_path}"));
            }
        }

        Command::new("explorer")
            .arg(&folder_path)
            .spawn()
            .map_err(|e| format!("Failed to open explorer: {e}"))?;
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}
