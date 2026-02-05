// Path: src-tauri/src/lib/agent/install.rs
// Description: Install the bundled WSL agent runtime into app local data

use crate::paths::wsl_convert::windows_to_wsl_path;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

const AGENT_BUNDLE_DIR: &str = "agent_bundle";
const AGENT_INSTALL_DIR: &str = "agent";
const AGENT_BINARY_FILE: &str = "im_agent";
const AGENT_VERSION_FILE: &str = "version.json";

#[derive(Debug, Clone)]
pub struct AgentBundlePaths {
    pub agent_dir_windows: PathBuf,
    pub agent_dir_wsl: String,
    pub log_dir_windows: PathBuf,
    pub log_dir_wsl: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
struct AgentBundleVersion {
    version: String,
}

pub fn ensure_agent_bundle(
    resource_dir: &Path,
    app_local_data: &Path,
) -> Result<AgentBundlePaths, String> {
    let bundle_dir = resolve_bundle_dir(resource_dir)?;

    let version_path = bundle_dir.join(AGENT_VERSION_FILE);
    let version = read_version(&version_path)?;

    let agent_dir_windows = app_local_data.join(AGENT_INSTALL_DIR);
    let installed_version_path = agent_dir_windows.join(AGENT_VERSION_FILE);
    let installed_version = read_installed_version(&installed_version_path);

    let should_install = installed_version.as_deref() != Some(version.as_str())
        || !agent_dir_windows.join(AGENT_BINARY_FILE).is_file();

    if should_install {
        install_bundle(&bundle_dir, &agent_dir_windows)?;
    }

    let log_dir_windows = app_local_data.join("logs");
    if let Err(err) = fs::create_dir_all(&log_dir_windows) {
        return Err(format!("Failed to create log directory: {err}"));
    }

    let agent_dir_wsl = windows_to_wsl_path(&path_to_string(&agent_dir_windows)?)
        .ok_or_else(|| "Failed to convert agent directory to WSL path".to_string())?;
    let log_dir_wsl = windows_to_wsl_path(&path_to_string(&log_dir_windows)?)
        .ok_or_else(|| "Failed to convert log directory to WSL path".to_string())?;

    Ok(AgentBundlePaths {
        agent_dir_windows,
        agent_dir_wsl,
        log_dir_windows,
        log_dir_wsl,
        version,
    })
}

fn resolve_bundle_dir(resource_dir: &Path) -> Result<PathBuf, String> {
    let direct = resource_dir.join(AGENT_BUNDLE_DIR);
    if direct.is_dir() {
        return Ok(direct);
    }

    let nested = resource_dir.join("resources").join(AGENT_BUNDLE_DIR);
    if nested.is_dir() {
        return Ok(nested);
    }

    Err(format!(
        "Agent bundle resources missing. Tried: {}, {}",
        direct.display(),
        nested.display()
    ))
}

fn install_bundle(bundle_dir: &Path, agent_dir_windows: &Path) -> Result<(), String> {
    let temp_dir = agent_dir_windows.with_extension("tmp");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)
            .map_err(|err| format!("Failed to clear temp agent bundle: {err}"))?;
    }

    fs::create_dir_all(&temp_dir)
        .map_err(|err| format!("Failed to create agent bundle temp dir: {err}"))?;

    copy_required(bundle_dir, &temp_dir, AGENT_BINARY_FILE)?;
    copy_required(bundle_dir, &temp_dir, AGENT_VERSION_FILE)?;

    if agent_dir_windows.exists() {
        fs::remove_dir_all(agent_dir_windows)
            .map_err(|err| format!("Failed to remove old agent bundle: {err}"))?;
    }

    fs::rename(&temp_dir, agent_dir_windows)
        .map_err(|err| format!("Failed to install agent bundle: {err}"))?;

    Ok(())
}

fn copy_required(bundle_dir: &Path, target_dir: &Path, file_name: &str) -> Result<(), String> {
    let source = bundle_dir.join(file_name);
    if !source.is_file() {
        return Err(format!("Agent bundle missing required file: {file_name}"));
    }
    let dest = target_dir.join(file_name);
    fs::copy(&source, &dest).map_err(|err| format!("Failed to copy {file_name}: {err}"))?;
    Ok(())
}

fn read_installed_version(path: &Path) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    match read_version(path) {
        Ok(version) => Some(version),
        Err(_) => None,
    }
}

fn read_version(path: &Path) -> Result<String, String> {
    let contents = fs::read_to_string(path)
        .map_err(|err| format!("Failed to read agent bundle version: {err}"))?;
    let parsed: AgentBundleVersion = serde_json::from_str(&contents)
        .map_err(|err| format!("Failed to parse agent bundle version: {err}"))?;
    let trimmed = parsed.version.trim();
    if trimmed.is_empty() {
        return Err("Agent bundle version is empty".to_string());
    }
    Ok(trimmed.to_string())
}

fn path_to_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .ok_or_else(|| "Path contains invalid UTF-8".to_string())
        .map(|value| value.to_string())
}
