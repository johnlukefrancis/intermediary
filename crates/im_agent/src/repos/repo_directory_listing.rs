// Path: crates/im_agent/src/repos/repo_directory_listing.rs
// Description: Lazy repo-relative directory listing for file explorer views

use std::io;
use std::path::{Component, Path};

use tokio::fs;

use crate::bundles::ignore_rules::should_ignore_entry;
use crate::error::AgentError;
use crate::source_control::ensure_no_git_component;

#[derive(Debug, Clone)]
pub struct RepoDirectoryListing {
    pub path: String,
    pub dirs: Vec<String>,
    pub files: Vec<String>,
}

pub async fn list_repo_directory(
    repo_root: &str,
    relative_path: &str,
) -> Result<RepoDirectoryListing, AgentError> {
    let normalized = normalize_directory_path(relative_path)?;
    // The repository's own Git directory is not part of the worktree this
    // listing describes, at any depth and whichever case the filesystem
    // spells it in. Refusing the request here is what keeps the explorer from
    // ever showing a path a drop, move, or rename would then be refused for.
    ensure_no_git_component(&normalized)?;
    let root_path = Path::new(repo_root);
    let target_path = if normalized.is_empty() {
        root_path.to_path_buf()
    } else {
        root_path.join(&normalized)
    };

    let canonical_root = fs::canonicalize(root_path)
        .await
        .map_err(|err| AgentError::internal(format!("Failed to resolve repo root: {err}")))?;
    let canonical_target =
        fs::canonicalize(&target_path)
            .await
            .map_err(|err| match err.kind() {
                io::ErrorKind::NotFound => {
                    AgentError::new("DIR_NOT_FOUND", "Directory does not exist")
                }
                _ => AgentError::internal(format!("Failed to resolve directory: {err}")),
            })?;

    if !canonical_target.starts_with(&canonical_root) {
        return Err(AgentError::new(
            "INVALID_PATH",
            "Path escapes configured repo root",
        ));
    }

    let metadata = fs::metadata(&canonical_target)
        .await
        .map_err(|err| AgentError::internal(format!("Failed to stat directory: {err}")))?;
    if !metadata.is_dir() {
        return Err(AgentError::new("NOT_DIRECTORY", "Path is not a directory"));
    }

    let mut dirs = Vec::new();
    let mut files = Vec::new();
    let mut entries = fs::read_dir(&canonical_target)
        .await
        .map_err(|err| AgentError::internal(format!("Failed to read directory: {err}")))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|err| AgentError::internal(format!("Failed to read directory entry: {err}")))?
    {
        let file_type = entry
            .file_type()
            .await
            .map_err(|err| AgentError::internal(format!("Failed to read entry type: {err}")))?;
        if file_type.is_symlink() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let child_path = join_relative_path(&normalized, &name);
        if file_type.is_dir() {
            if name.eq_ignore_ascii_case(".git") {
                continue;
            }
            dirs.push(child_path);
        } else if file_type.is_file() && !should_ignore_entry(&name, false) {
            files.push(child_path);
        }
    }

    dirs.sort();
    files.sort();

    Ok(RepoDirectoryListing {
        path: normalized,
        dirs,
        files,
    })
}

fn join_relative_path(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}

/// Normalizes one UI-supplied repo-relative directory path to the
/// slash-joined form every repo path uses on the wire. `""` and `"."` both
/// mean the worktree root and normalize to `""`.
pub fn normalize_directory_path(relative_path: &str) -> Result<String, AgentError> {
    let trimmed = relative_path.trim();
    if trimmed.is_empty() || trimmed == "." {
        return Ok(String::new());
    }
    if trimmed.contains('\\') {
        return Err(AgentError::new(
            "INVALID_PATH",
            "Backslashes not allowed in relative paths",
        ));
    }

    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Err(AgentError::new(
            "INVALID_PATH",
            "Absolute paths not allowed",
        ));
    }

    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => parts.push(value.to_string_lossy().to_string()),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(AgentError::new(
                    "INVALID_PATH",
                    "Path traversal not allowed",
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(AgentError::new(
                    "INVALID_PATH",
                    "Absolute paths not allowed",
                ));
            }
        }
    }

    if parts.is_empty() {
        return Ok(String::new());
    }

    Ok(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::list_repo_directory;
    use std::fs;
    use tempfile::tempdir;

    #[cfg(unix)]
    use std::os::unix::fs as unix_fs;

    #[tokio::test]
    async fn lists_root_directories_and_files() {
        let dir = tempdir().expect("temp repo");
        let root = dir.path();
        fs::create_dir_all(root.join("app/src")).expect("create app");
        fs::write(root.join("README.md"), "docs").expect("write readme");
        fs::write(root.join(".env"), "secret").expect("write ignored file");
        fs::create_dir_all(root.join(".git/hooks")).expect("create git dir");

        let result = list_repo_directory(root.to_str().expect("root"), "")
            .await
            .expect("list root");

        assert_eq!(result.path, "");
        assert_eq!(
            result.dirs,
            vec!["app".to_string()],
            "the repository's own Git directory is never listed"
        );
        assert_eq!(result.files, vec!["README.md".to_string()]);
    }

    #[tokio::test]
    async fn lists_nested_directories_and_files_as_repo_relative_paths() {
        let dir = tempdir().expect("temp repo");
        let root = dir.path();
        fs::create_dir_all(root.join("app/src/components")).expect("create dirs");
        fs::write(root.join("app/src/main.ts"), "const x = 1;").expect("write file");

        let result = list_repo_directory(root.to_str().expect("root"), "app/src")
            .await
            .expect("list nested");

        assert_eq!(result.path, "app/src");
        assert_eq!(result.dirs, vec!["app/src/components".to_string()]);
        assert_eq!(result.files, vec!["app/src/main.ts".to_string()]);
    }

    /// The Git directory is not a directory of this worktree: asking for it,
    /// for anything under it, or for its case alias is refused rather than
    /// answered with a listing the rest of the product would refuse to act on.
    #[tokio::test]
    async fn rejects_the_git_directory_at_any_depth() {
        let dir = tempdir().expect("temp repo");
        fs::create_dir_all(dir.path().join(".git/hooks")).expect("git dir");
        fs::create_dir_all(dir.path().join("app/.GIT")).expect("nested git dir");

        for path in [".git", ".git/hooks", "app/.GIT"] {
            let err = list_repo_directory(dir.path().to_str().expect("root"), path)
                .await
                .expect_err("git directory");
            assert_eq!(err.code(), "INVALID_PATH", "{path}");
        }
    }

    #[tokio::test]
    async fn rejects_traversal_paths() {
        let dir = tempdir().expect("temp repo");
        let err = list_repo_directory(dir.path().to_str().expect("root"), "../outside")
            .await
            .expect_err("traversal should fail");

        assert_eq!(err.code(), "INVALID_PATH");
    }

    #[tokio::test]
    async fn rejects_file_paths() {
        let dir = tempdir().expect("temp repo");
        fs::write(dir.path().join("README.md"), "docs").expect("write readme");

        let err = list_repo_directory(dir.path().to_str().expect("root"), "README.md")
            .await
            .expect_err("file should fail");

        assert_eq!(err.code(), "NOT_DIRECTORY");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn skips_symlink_children() {
        let dir = tempdir().expect("temp repo");
        let outside = tempdir().expect("outside");
        fs::create_dir_all(dir.path().join("real")).expect("real dir");
        unix_fs::symlink(outside.path(), dir.path().join("linked")).expect("symlink dir");

        let result = list_repo_directory(dir.path().to_str().expect("root"), "")
            .await
            .expect("list root");

        assert_eq!(result.dirs, vec!["real".to_string()]);
    }
}
