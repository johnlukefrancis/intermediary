// Path: src-tauri/src/lib/commands/file_opener.rs
// Description: Reveal files in file manager or open with default application

use crate::config::types::RepoRoot;
use std::path::{Component, Path};
use std::process::Command;

use super::file_manager::resolve_host_path;

fn validate_relative_path(relative_path: &str) -> Result<(), String> {
    if relative_path.trim().is_empty() {
        return Err("Relative path cannot be empty".to_string());
    }

    let path = Path::new(relative_path);
    if path.is_absolute() {
        return Err("Absolute paths are not allowed".to_string());
    }

    let mut has_normal = false;
    for component in path.components() {
        match component {
            Component::ParentDir => {
                return Err("Path traversal is not allowed".to_string());
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err("Absolute paths are not allowed".to_string());
            }
            Component::Normal(_) => has_normal = true,
            Component::CurDir => {}
        }
    }

    if !has_normal {
        return Err("Relative path cannot be empty".to_string());
    }

    Ok(())
}

fn resolve_host_file_path(root: &RepoRoot, relative_path: &str) -> Result<String, String> {
    let normalized_relative = relative_path.trim().replace('\\', "/");
    validate_relative_path(&normalized_relative)?;

    let absolute_path = build_absolute_repo_path(root, &normalized_relative)?;
    resolve_host_path(&absolute_path)
}

fn build_absolute_repo_path(root: &RepoRoot, normalized_relative: &str) -> Result<String, String> {
    let root_path = root.path().trim().to_string();
    if root_path.is_empty() {
        return Err("Repo root path cannot be empty".to_string());
    }

    match root {
        RepoRoot::Wsl { .. } => {
            let trimmed_root = root_path.trim_end_matches('/');
            if trimmed_root.is_empty() {
                return Ok(format!("/{}", normalized_relative));
            }
            Ok(format!("{trimmed_root}/{normalized_relative}"))
        }
        RepoRoot::Host { .. } => Ok(Path::new(&root_path)
            .join(normalized_relative)
            .to_string_lossy()
            .to_string()),
    }
}

/// Reveal a file highlighted in the OS file manager.
///
/// # Errors
/// Returns an error if path validation fails or the platform launcher fails.
#[tauri::command]
pub async fn reveal_in_file_manager(root: RepoRoot, relative_path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let host_path = resolve_host_file_path(&root, &relative_path)?;
        let path = Path::new(&host_path);
        if !path.exists() || path.is_dir() {
            return Err(format!("File does not exist: {host_path}"));
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("explorer")
                .args(["/select,", &host_path])
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
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}

/// Open a file with the OS default application.
///
/// # Errors
/// Returns an error if path validation fails or the platform launcher fails.
#[tauri::command]
pub async fn open_file(root: RepoRoot, relative_path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let host_path = resolve_host_file_path(&root, &relative_path)?;
        let path = Path::new(&host_path);
        if !path.exists() || path.is_dir() {
            return Err(format!("File does not exist: {host_path}"));
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("explorer")
                .arg(&host_path)
                .spawn()
                .map_err(|e| format!("Failed to open file: {e}"))?;
            return Ok::<(), String>(());
        }

        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .arg(&host_path)
                .spawn()
                .map_err(|e| format!("Failed to open file: {e}"))?;
            return Ok::<(), String>(());
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            Command::new("xdg-open")
                .arg(&host_path)
                .spawn()
                .map_err(|e| format!("Failed to open file: {e}"))?;
            return Ok::<(), String>(());
        }

        #[allow(unreachable_code)]
        Err("open_file is not supported on this platform".to_string())
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_relative_path_rejects_invalid_paths() {
        let invalid_paths = [
            "",
            ".",
            "..",
            "../outside",
            "/abs/path",
            "C:\\Windows\\System32",
            "folder/../escape",
            "folder\\file.txt",
        ];
        for path in invalid_paths {
            assert!(
                validate_relative_path(path).is_err(),
                "path should be rejected: {path}"
            );
        }
    }

    #[test]
    fn validate_relative_path_allows_normal_paths() {
        let valid_paths = ["README.md", "docs/prd.md", "src/lib/file.ts"];
        for path in valid_paths {
            assert!(
                validate_relative_path(path).is_ok(),
                "path should be accepted: {path}"
            );
        }
    }

    #[test]
    fn build_absolute_repo_path_preserves_wsl_root_shape() {
        let root = RepoRoot::Wsl {
            path: "/home/john/repo".to_string(),
        };
        let absolute =
            build_absolute_repo_path(&root, "docs/prd.md").expect("wsl path should resolve");
        assert_eq!(absolute, "/home/john/repo/docs/prd.md");
    }
}
