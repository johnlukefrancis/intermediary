// Path: src-tauri/src/lib/commands/file_opener.rs
// Description: Reveal files in file manager or open with default application

use crate::config::types::RepoRoot;
use std::path::Path;
use std::process::Command;
use tauri::AppHandle;

#[cfg(target_os = "windows")]
use super::file_manager::explorer_file_arg;
use super::file_manager::{is_windows_unc_path, resolve_host_path};
use super::file_open_policy::open_paths_by_policy;
use super::file_opener_paths::{resolve_host_file_path, resolve_host_file_paths};
use super::wsl_distro::resolve_runtime_wsl_distro;

fn reveal_existing_host_file(host_path: String) -> Result<(), String> {
    let path = Path::new(&host_path);
    if !is_windows_unc_path(&host_path) && (!path.exists() || path.is_dir()) {
        return Err(format!("File does not exist: {host_path}"));
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(explorer_file_arg(&host_path))
            .spawn()
            .map_err(|e| format!("Failed to open Explorer: {e}"))?;
        return Ok::<(), String>(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-R", &host_path])
            .spawn()
            .map_err(|e| format!("Failed to reveal in Finder: {e}"))?;
        return Ok::<(), String>(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let parent = path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| host_path.clone());
        Command::new("xdg-open")
            .arg(&parent)
            .spawn()
            .map_err(|e| format!("Failed to open file manager: {e}"))?;
        return Ok::<(), String>(());
    }

    #[allow(unreachable_code)]
    Err("reveal_in_file_manager is not supported on this platform".to_string())
}

#[tauri::command]
pub async fn reveal_in_file_manager(
    app: AppHandle,
    root: RepoRoot,
    relative_path: String,
    distro_override: Option<String>,
) -> Result<(), String> {
    let distro_override = resolve_runtime_wsl_distro(&app, distro_override.as_deref());
    tauri::async_runtime::spawn_blocking(move || {
        let host_path = resolve_host_file_path(&root, &relative_path, distro_override.as_deref())?;
        reveal_existing_host_file(host_path)
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}

#[tauri::command]
pub async fn reveal_host_file_in_file_manager(
    app: AppHandle,
    host_path: String,
    distro_override: Option<String>,
) -> Result<(), String> {
    let host_path = host_path.trim().to_string();
    if host_path.is_empty() {
        return Err("File path cannot be empty".to_string());
    }

    let distro_override = resolve_runtime_wsl_distro(&app, distro_override.as_deref());
    tauri::async_runtime::spawn_blocking(move || {
        let resolved_host_path = resolve_host_path(&host_path, distro_override.as_deref())?;
        reveal_existing_host_file(resolved_host_path)
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}

#[tauri::command]
pub async fn open_file(
    app: AppHandle,
    root: RepoRoot,
    relative_path: String,
    distro_override: Option<String>,
) -> Result<(), String> {
    let distro_override = resolve_runtime_wsl_distro(&app, distro_override.as_deref());
    tauri::async_runtime::spawn_blocking(move || {
        let relative_paths = vec![relative_path];
        open_paths_by_policy(
            &relative_paths,
            &resolve_host_file_paths(&root, &relative_paths, distro_override.as_deref())?,
        )
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}

#[tauri::command]
pub async fn open_files(
    app: AppHandle,
    root: RepoRoot,
    relative_paths: Vec<String>,
    distro_override: Option<String>,
) -> Result<(), String> {
    let distro_override = resolve_runtime_wsl_distro(&app, distro_override.as_deref());
    tauri::async_runtime::spawn_blocking(move || {
        open_paths_by_policy(
            &relative_paths,
            &resolve_host_file_paths(&root, &relative_paths, distro_override.as_deref())?,
        )
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}
