// Path: src-tauri/src/lib/commands/file_manager.rs
// Description: Open folders in the host OS file manager

#[cfg(target_os = "windows")]
use crate::paths::wsl_convert::{run_wslpath, wsl_to_windows_path};
use std::path::Path;
use std::process::Command;
use tauri::AppHandle;

use super::wsl_distro::resolve_runtime_wsl_distro;

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn explorer_folder_arg(host_path: &str) -> String {
    format!("/e,{host_path}")
}

/// Build the single Explorer argument used to reveal a file in its parent folder.
///
/// Explorer's comma-delimited selection switch must stay on the same argument
/// boundary as the target path. The file form intentionally uses `/select,`
/// without the folder-only `/e,` switch: Explorer treats that as the reveal
/// contract for an existing file, including Windows drive and UNC paths.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn explorer_file_arg(host_path: &str) -> String {
    format!("/select,{host_path}")
}

/// Open a folder in the host OS file manager.
///
/// # Arguments
/// * `folder_path` - Absolute folder path to open.
///   On Windows, absolute WSL paths are resolved to host-visible paths.
///
/// # Errors
/// Returns an error if the path is empty or the platform launcher fails.
#[tauri::command]
pub async fn open_in_file_manager(
    app: AppHandle,
    folder_path: String,
    distro_override: Option<String>,
) -> Result<(), String> {
    let folder_path = folder_path.trim().to_string();
    if folder_path.is_empty() {
        return Err("Folder path cannot be empty".to_string());
    }

    let distro_override = resolve_runtime_wsl_distro(&app, distro_override.as_deref());
    tauri::async_runtime::spawn_blocking(move || {
        let host_path = resolve_host_path(&folder_path, distro_override.as_deref())?;
        let path = Path::new(&host_path);
        if !is_windows_unc_path(&host_path) && !path.is_dir() {
            return Err(format!("Folder does not exist: {host_path}"));
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("explorer")
                .arg(explorer_folder_arg(&host_path))
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

pub(crate) fn is_windows_unc_path(path: &str) -> bool {
    cfg!(target_os = "windows") && path.starts_with(r"\\")
}

#[cfg(target_os = "windows")]
pub(crate) fn resolve_host_path(
    folder_path: &str,
    distro_override: Option<&str>,
) -> Result<String, String> {
    if !folder_path.starts_with('/') {
        return Ok(folder_path.to_string());
    }

    if let Some(windows_path) = wsl_to_windows_path(folder_path) {
        return Ok(windows_path);
    }

    run_wslpath(folder_path, distro_override).map_err(|error| {
        format!("Failed to resolve WSL folder path '{folder_path}' to a Windows path: {error}")
    })
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn resolve_host_path(
    folder_path: &str,
    _distro_override: Option<&str>,
) -> Result<String, String> {
    Ok(folder_path.to_string())
}

#[cfg(test)]
mod tests {
    use super::{explorer_file_arg, explorer_folder_arg};

    #[test]
    fn explorer_folder_arg_keeps_switch_and_windows_path_together() {
        assert_eq!(
            explorer_folder_arg(r"C:\Worktrees\Windows Project"),
            r"/e,C:\Worktrees\Windows Project"
        );
    }

    #[test]
    fn explorer_folder_arg_keeps_switch_and_unc_path_together() {
        assert_eq!(
            explorer_folder_arg(r"\\wsl.localhost\Ubuntu\home\johnf\code"),
            r"/e,\\wsl.localhost\Ubuntu\home\johnf\code"
        );
    }

    #[test]
    fn explorer_file_arg_keeps_selection_and_windows_path_together() {
        assert_eq!(
            explorer_file_arg(r"C:\Worktrees\Windows Project\Docs\Guide Notes.md"),
            r"/select,C:\Worktrees\Windows Project\Docs\Guide Notes.md"
        );
    }

    #[test]
    fn explorer_file_arg_keeps_selection_and_unc_path_together() {
        assert_eq!(
            explorer_file_arg(r"\\wsl.localhost\Ubuntu\home\johnf\code\Docs\Guide.md"),
            r"/select,\\wsl.localhost\Ubuntu\home\johnf\code\Docs\Guide.md"
        );
    }
}
