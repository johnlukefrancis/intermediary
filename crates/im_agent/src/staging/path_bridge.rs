// Path: crates/im_agent/src/staging/path_bridge.rs
// Description: Staging path bridging between WSL and host platform layouts

use std::path::{Path, PathBuf};

use crate::error::AgentError;

#[derive(Debug, Clone)]
pub struct PathBridgeConfig {
    pub staging_host_root: String,
    pub staging_wsl_root: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StagingRootKind {
    Wsl,
    Host,
}

pub struct StagedPaths {
    pub host_path: String,
    pub wsl_path: Option<String>,
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

    let host_root = normalize_host_root(&config.staging_host_root);

    let wsl_path = match staging_kind {
        StagingRootKind::Wsl => {
            let wsl_root_str = config.staging_wsl_root.as_deref().ok_or_else(|| {
                AgentError::new(
                    "MISSING_WSL_ROOT",
                    "WSL staging root is required for WSL staging kind",
                )
            })?;
            let wsl_root = normalize_wsl_root(wsl_root_str);
            Some(join_slash_path(&wsl_root, repo_id, &normalized_relative))
        }
        StagingRootKind::Host => {
            let host_target = join_host_path(&host_root, repo_id, &normalized_relative);
            windows_to_wsl(&host_target).map(Some).unwrap_or(None)
        }
    };

    let host_path = match staging_kind {
        StagingRootKind::Wsl => {
            let wsl_root_str = config.staging_wsl_root.as_deref().ok_or_else(|| {
                AgentError::new(
                    "MISSING_WSL_ROOT",
                    "WSL staging root is required for WSL staging kind",
                )
            })?;
            let wsl_root = normalize_wsl_root(wsl_root_str);
            let source = join_slash_path(&wsl_root, repo_id, &normalized_relative);
            wsl_to_windows(&source)?
        }
        StagingRootKind::Host => join_host_path(&host_root, repo_id, &normalized_relative),
    };

    Ok(StagedPaths {
        host_path,
        wsl_path,
    })
}

pub fn staging_local_path<'a>(
    staged_paths: &'a StagedPaths,
    staging_kind: StagingRootKind,
) -> Result<&'a str, AgentError> {
    match staging_kind {
        StagingRootKind::Wsl => staged_paths.wsl_path.as_deref().ok_or_else(|| {
            AgentError::new(
                "MISSING_WSL_ROOT",
                "WSL path is not available for staging",
            )
        }),
        StagingRootKind::Host => Ok(&staged_paths.host_path),
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

fn normalize_host_root(root: &str) -> String {
    if root.contains('\\') || (root.len() >= 2 && root.as_bytes()[1] == b':') {
        let trimmed = root.trim_end_matches('\\');
        if trimmed.to_ascii_lowercase().ends_with("\\files") {
            trimmed.to_string()
        } else {
            format!("{trimmed}\\files")
        }
    } else {
        let trimmed = root.trim_end_matches('/');
        if trimmed.ends_with("/files") {
            trimmed.to_string()
        } else {
            format!("{trimmed}/files")
        }
    }
}

fn join_slash_path(root: &str, repo_id: &str, relative: &str) -> String {
    Path::new(root)
        .join(repo_id)
        .join(relative)
        .to_string_lossy()
        .to_string()
}

fn join_host_path(root: &str, repo_id: &str, relative: &str) -> String {
    if root.contains('\\') || (root.len() >= 2 && root.as_bytes()[1] == b':') {
        let relative_windows = relative.replace('/', "\\");
        format!("{root}\\{repo_id}\\{relative_windows}")
    } else {
        Path::new(root)
            .join(repo_id)
            .join(relative)
            .to_string_lossy()
            .to_string()
    }
}

pub fn ensure_path_dir(path: &str) -> PathBuf {
    Path::new(path)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(path))
}
