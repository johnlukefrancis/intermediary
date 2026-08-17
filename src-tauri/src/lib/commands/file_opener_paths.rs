// Path: src-tauri/src/lib/commands/file_opener_paths.rs
// Description: Resolve repo-relative file paths to host-visible paths

use crate::config::types::RepoRoot;
use std::path::{Component, Path};

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

pub(crate) fn resolve_host_file_path(
    root: &RepoRoot,
    relative_path: &str,
    distro_override: Option<&str>,
) -> Result<String, String> {
    let normalized_relative = relative_path.trim().replace('\\', "/");
    validate_relative_path(&normalized_relative)?;

    let absolute_path = build_absolute_repo_path(root, &normalized_relative)?;
    resolve_host_path(&absolute_path, distro_override)
}

pub(crate) fn resolve_host_file_paths(
    root: &RepoRoot,
    relative_paths: &[String],
    distro_override: Option<&str>,
) -> Result<Vec<String>, String> {
    if relative_paths.is_empty() {
        return Err("No files provided".to_string());
    }

    let mut host_paths = Vec::with_capacity(relative_paths.len());
    for relative_path in relative_paths {
        let host_path = resolve_host_file_path(root, relative_path, distro_override)?;
        let path = Path::new(&host_path);
        if !path.exists() || path.is_dir() {
            return Err(format!("File does not exist: {host_path}"));
        }
        host_paths.push(host_path);
    }

    Ok(host_paths)
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
        RepoRoot::Host { .. } => {
            let (host_root, host_relative) = if cfg!(target_os = "windows") {
                (
                    root_path.replace('/', "\\"),
                    normalized_relative.replace('/', "\\"),
                )
            } else {
                (root_path, normalized_relative.to_string())
            };
            Ok(Path::new(&host_root)
                .join(host_relative)
                .to_string_lossy()
                .to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::build_absolute_repo_path;
    use crate::config::types::RepoRoot;

    #[cfg(target_os = "windows")]
    #[test]
    fn host_repo_file_path_uses_windows_separators() {
        let root = RepoRoot::Host {
            path: "C:/Worktrees/Windows Project".to_string(),
        };

        assert_eq!(
            build_absolute_repo_path(&root, "Docs/Guide Notes.md"),
            Ok(r"C:\Worktrees\Windows Project\Docs\Guide Notes.md".to_string())
        );
    }

    #[test]
    fn wsl_repo_file_path_keeps_posix_separators() {
        let root = RepoRoot::Wsl {
            path: "/home/johnf/code".to_string(),
        };

        assert_eq!(
            build_absolute_repo_path(&root, "Docs/Guide Notes.md"),
            Ok("/home/johnf/code/Docs/Guide Notes.md".to_string())
        );
    }
}
