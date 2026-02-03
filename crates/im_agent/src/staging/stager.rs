// Path: crates/im_agent/src/staging/stager.rs
// Description: Atomic staging of files into the Windows-accessible directory

use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::fs;

use crate::error::AgentError;
use crate::staging::path_bridge::{build_staged_paths, ensure_path_dir, PathBridgeConfig};

#[derive(Debug, Clone)]
pub struct StageResult {
    pub wsl_path: String,
    pub windows_path: String,
    pub bytes_copied: u64,
    pub mtime_ms: u64,
}

pub async fn stage_file(
    config: &PathBridgeConfig,
    repo_id: &str,
    repo_root: &str,
    relative_path: &str,
) -> Result<StageResult, AgentError> {
    validate_relative_path(relative_path)?;

    let source_path = Path::new(repo_root).join(relative_path);
    let staged_paths = build_staged_paths(config, repo_id, relative_path)?;

    let dest_dir = ensure_path_dir(&staged_paths.wsl_path);
    fs::create_dir_all(&dest_dir)
        .await
        .map_err(|err| AgentError::internal(format!("Failed to create staging dir: {err}")))?;

    let temp_path = temp_path_for(Path::new(&staged_paths.wsl_path));

    let bytes_copied = match fs::copy(&source_path, &temp_path).await {
        Ok(bytes) => bytes,
        Err(err) => {
            let _ = fs::remove_file(&temp_path).await;
            return Err(AgentError::internal(format!(
                "Failed to stage file: {err}"
            )));
        }
    };

    if let Err(err) = fs::rename(&temp_path, &staged_paths.wsl_path).await {
        let _ = fs::remove_file(&temp_path).await;
        return Err(AgentError::internal(format!(
            "Failed to finalize staged file: {err}"
        )));
    }

    let metadata = fs::metadata(&staged_paths.wsl_path)
        .await
        .map_err(|err| AgentError::internal(format!("Failed to stat staged file: {err}")))?;

    let mtime_ms = metadata
        .modified()
        .ok()
        .and_then(|mtime| mtime.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);

    Ok(StageResult {
        wsl_path: staged_paths.wsl_path,
        windows_path: staged_paths.windows_path,
        bytes_copied,
        mtime_ms,
    })
}

fn temp_path_for(dest_path: &Path) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = dest_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "staged".to_string());
    let temp_name = format!("{file_name}.{suffix}.tmp");
    dest_path.with_file_name(temp_name)
}

pub fn validate_relative_path(relative_path: &str) -> Result<(), AgentError> {
    if relative_path.trim().is_empty() {
        return Err(AgentError::new(
            "INVALID_PATH",
            "Empty relative paths not allowed",
        ));
    }

    if relative_path == "." {
        return Err(AgentError::new(
            "INVALID_PATH",
            "Dot relative paths not allowed",
        ));
    }

    if relative_path.contains('\\') {
        return Err(AgentError::new(
            "INVALID_PATH",
            "Backslashes not allowed in relative paths",
        ));
    }

    let path = Path::new(relative_path);
    if path.is_absolute() {
        return Err(AgentError::new(
            "INVALID_PATH",
            "Absolute paths not allowed",
        ));
    }

    let mut has_normal = false;
    for component in path.components() {
        match component {
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
            Component::Normal(_) => has_normal = true,
            Component::CurDir => {}
        }
    }

    if !has_normal {
        return Err(AgentError::new(
            "INVALID_PATH",
            "Empty relative paths not allowed",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn new_temp_root() -> (TempDir, PathBridgeConfig) {
        let root = TempDir::new().expect("tempdir");
        let staging_root = root.path().join("staging");
        let config = PathBridgeConfig {
            staging_wsl_root: staging_root.to_string_lossy().to_string(),
            staging_win_root: "C:\\staging".to_string(),
        };
        (root, config)
    }

    #[tokio::test]
    async fn rejects_traversal_paths() {
        let (root, config) = new_temp_root();
        let repo_root = root.path().join("repo");
        fs::create_dir_all(&repo_root).expect("repo root");
        let repo_root_str = repo_root.to_string_lossy().to_string();

        let invalid_paths = vec![
            "",
            ".",
            "..",
            "../outside",
            "./",
            "/abs/path",
            "C:\\Windows\\System32",
            "..\\windows",
            "folder/../escape",
            "folder\\file.txt",
        ];

        for path in invalid_paths {
            let result = stage_file(
                &config,
                "repo",
                &repo_root_str,
                path,
            )
            .await;
            let err = result.expect_err("expected invalid path error");
            assert_eq!(err.code(), "INVALID_PATH", "path: {path}");
        }
    }
}
