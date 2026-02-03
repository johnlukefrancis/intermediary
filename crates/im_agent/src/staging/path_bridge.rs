// Path: crates/im_agent/src/staging/path_bridge.rs
// Description: Staging path bridging between WSL and Windows layouts

use std::path::{Path, PathBuf};

use crate::error::AgentError;

#[derive(Debug, Clone)]
pub struct PathBridgeConfig {
    pub staging_wsl_root: String,
    pub staging_win_root: String,
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
    let normalized_relative = relative_path.replace('\\', "/");
    let wsl_root = normalize_wsl_root(&config.staging_wsl_root);
    let win_root = normalize_win_root(&config.staging_win_root);

    let wsl_path = Path::new(&wsl_root)
        .join(repo_id)
        .join(&normalized_relative)
        .to_string_lossy()
        .to_string();

    let windows_path = format!(
        "{}\\{}\\{}",
        win_root,
        repo_id,
        normalized_relative.replace('/', "\\")
    );

    Ok(StagedPaths {
        wsl_path,
        windows_path,
    })
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

pub fn ensure_path_dir(path: &str) -> PathBuf {
    Path::new(path)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(path))
}
