// Path: crates/im_agent/src/staging/path_bridge.rs
// Description: Staging path bridging between WSL and Windows layouts

use std::path::{Path, PathBuf};

use crate::error::AgentError;

#[derive(Debug, Clone)]
pub struct PathBridgeConfig {
    pub staging_wsl_root: String,
    pub staging_win_root: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StagingRootKind {
    Wsl,
    Windows,
}

pub struct StagedPaths {
    pub wsl_path: String,
    pub windows_path: String,
}

pub fn build_staged_paths(
    config: &PathBridgeConfig,
    repo_id: &str,
    relative_path: &str,
) -> Result<StagedPaths, AgentError> {
    build_staged_paths_for_kind(config, repo_id, relative_path, StagingRootKind::Wsl)
}

pub fn build_staged_paths_for_kind(
    config: &PathBridgeConfig,
    repo_id: &str,
    relative_path: &str,
    staging_kind: StagingRootKind,
) -> Result<StagedPaths, AgentError> {
    let normalized_relative = relative_path.replace('\\', "/");
    let wsl_root = normalize_wsl_root(&config.staging_wsl_root);
    let win_root = normalize_win_root(&config.staging_win_root);

    let wsl_path = match staging_kind {
        StagingRootKind::Wsl => join_slash_path(&wsl_root, repo_id, &normalized_relative),
        StagingRootKind::Windows => {
            let windows_target = join_windows_path(&win_root, repo_id, &normalized_relative);
            windows_to_wsl(&windows_target).ok_or_else(|| {
                AgentError::new(
                    "INVALID_PATH",
                    format!(
                        "Cannot convert Windows staging path to WSL path: {}",
                        windows_target
                    ),
                )
            })?
        }
    };

    let windows_path = match staging_kind {
        StagingRootKind::Wsl => {
            let source = join_slash_path(&wsl_root, repo_id, &normalized_relative);
            wsl_to_windows(&source)?
        }
        StagingRootKind::Windows => join_windows_path(&win_root, repo_id, &normalized_relative),
    };

    Ok(StagedPaths {
        wsl_path,
        windows_path,
    })
}

pub fn staging_local_path(staged_paths: &StagedPaths, staging_kind: StagingRootKind) -> &str {
    match staging_kind {
        StagingRootKind::Wsl => &staged_paths.wsl_path,
        StagingRootKind::Windows => &staged_paths.windows_path,
    }
}

pub fn wsl_to_windows(wsl_path: &str) -> Result<String, AgentError> {
    let stripped = wsl_path.strip_prefix("/mnt/").ok_or_else(|| {
        AgentError::new(
            "INVALID_PATH",
            format!("Invalid WSL path for Windows conversion: {wsl_path}"),
        )
    })?;

    let mut parts = stripped.splitn(2, '/');
    let drive = parts.next().unwrap_or_default();
    let rest = parts.next().unwrap_or("");

    if drive.len() != 1 {
        return Err(AgentError::new(
            "INVALID_PATH",
            format!("Invalid WSL path for Windows conversion: {wsl_path}"),
        ));
    }

    let drive = drive.chars().next().unwrap().to_ascii_uppercase();
    let windows_rest = rest.replace('/', "\\");

    Ok(format!("{drive}:\\{windows_rest}"))
}

pub fn windows_to_wsl(windows_path: &str) -> Option<String> {
    let normalized = windows_path.trim().replace('/', "\\");
    let bytes = normalized.as_bytes();
    if bytes.len() < 2 || bytes[1] != b':' {
        return None;
    }

    let drive = (bytes[0] as char).to_ascii_lowercase();
    if !drive.is_ascii_alphabetic() {
        return None;
    }

    let suffix = normalized[2..].trim_start_matches('\\').replace('\\', "/");
    if suffix.is_empty() {
        return Some(format!("/mnt/{drive}"));
    }

    Some(format!("/mnt/{drive}/{suffix}"))
}

fn normalize_wsl_root(root: &str) -> String {
    let trimmed = root.trim_end_matches('/');
    if trimmed.ends_with("/files") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/files")
    }
}

fn normalize_win_root(root: &str) -> String {
    let trimmed = root.trim_end_matches('\\');
    if trimmed.to_ascii_lowercase().ends_with("\\files") {
        trimmed.to_string()
    } else {
        format!("{trimmed}\\files")
    }
}

fn join_slash_path(root: &str, repo_id: &str, relative: &str) -> String {
    Path::new(root)
        .join(repo_id)
        .join(relative)
        .to_string_lossy()
        .to_string()
}

fn join_windows_path(root: &str, repo_id: &str, relative: &str) -> String {
    let relative_windows = relative.replace('/', "\\");
    format!("{root}\\{repo_id}\\{relative_windows}")
}

pub fn ensure_path_dir(path: &str) -> PathBuf {
    Path::new(path)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(path))
}
