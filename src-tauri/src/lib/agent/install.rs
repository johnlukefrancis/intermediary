// Path: src-tauri/src/lib/agent/install.rs
// Description: Install bundled agent runtimes into app local data with platform-specific requirements

use super::install_runtime::{
    build_bundle_paths, install_bundle, installed_bundle_matches, read_installed_version,
    read_version,
};
use std::path::{Path, PathBuf};

const AGENT_BUNDLE_DIR: &str = "agent_bundle";
const AGENT_INSTALL_DIR: &str = "agent";
const WSL_AGENT_BINARY_FILE: &str = "im_agent";
#[cfg(target_os = "windows")]
const HOST_AGENT_BINARY_FILE: &str = "im_host_agent.exe";
#[cfg(not(target_os = "windows"))]
const HOST_AGENT_BINARY_FILE: &str = "im_host_agent";
const AGENT_VERSION_FILE: &str = "version.json";

#[derive(Debug, Clone)]
pub struct AgentBundlePaths {
    pub agent_dir_host: PathBuf,
    pub log_dir_host: PathBuf,
    pub host_agent_binary_host: PathBuf,
    pub wsl_agent_binary_host: Option<PathBuf>,
    pub version: String,
}

pub fn ensure_agent_bundle(
    resource_dir: &Path,
    app_local_data: &Path,
) -> Result<AgentBundlePaths, String> {
    let bundle_dir = resolve_bundle_dir(resource_dir)?;

    let version_path = bundle_dir.join(AGENT_VERSION_FILE);
    let version = read_version(&version_path)?;

    let agent_dir_host = app_local_data.join(AGENT_INSTALL_DIR);
    let installed_version_path = agent_dir_host.join(AGENT_VERSION_FILE);
    let installed_version = read_installed_version(&installed_version_path);

    let should_install = if installed_version.as_deref() != Some(version.as_str()) {
        true
    } else {
        !installed_bundle_matches(
            &bundle_dir,
            &agent_dir_host,
            AGENT_VERSION_FILE,
            WSL_AGENT_BINARY_FILE,
            HOST_AGENT_BINARY_FILE,
            requires_wsl_binary(),
        )?
    };

    if should_install {
        install_bundle(
            &bundle_dir,
            &agent_dir_host,
            AGENT_VERSION_FILE,
            WSL_AGENT_BINARY_FILE,
            HOST_AGENT_BINARY_FILE,
            requires_wsl_binary(),
        )?;
    }

    build_bundle_paths(
        agent_dir_host,
        app_local_data.join("logs"),
        version,
        WSL_AGENT_BINARY_FILE,
        HOST_AGENT_BINARY_FILE,
        requires_wsl_binary(),
    )
}

pub fn resolve_installed_agent_bundle(app_local_data: &Path) -> Result<AgentBundlePaths, String> {
    let agent_dir_host = app_local_data.join(AGENT_INSTALL_DIR);
    let version = read_version(&agent_dir_host.join(AGENT_VERSION_FILE))?;

    if !agent_dir_host.join(HOST_AGENT_BINARY_FILE).is_file() {
        return Err(format!(
            "Installed agent is missing required file: {HOST_AGENT_BINARY_FILE}"
        ));
    }
    if requires_wsl_binary() && !agent_dir_host.join(WSL_AGENT_BINARY_FILE).is_file() {
        return Err(format!(
            "Installed agent is missing required file: {WSL_AGENT_BINARY_FILE}"
        ));
    }

    build_bundle_paths(
        agent_dir_host,
        app_local_data.join("logs"),
        version,
        WSL_AGENT_BINARY_FILE,
        HOST_AGENT_BINARY_FILE,
        requires_wsl_binary(),
    )
}

pub fn resolve_launch_bundle(
    resource_dir: &Path,
    app_local_data: &Path,
    prefer_installed: bool,
) -> Result<AgentBundlePaths, String> {
    if prefer_installed {
        if let Ok(bundle) = resolve_installed_agent_bundle(app_local_data) {
            return Ok(bundle);
        }
    }

    ensure_agent_bundle(resource_dir, app_local_data)
}

fn resolve_bundle_dir(resource_dir: &Path) -> Result<PathBuf, String> {
    let mut tried: Vec<PathBuf> = Vec::new();
    let mut candidates: Vec<PathBuf> = Vec::new();

    candidates.push(resource_dir.join(AGENT_BUNDLE_DIR));
    candidates.push(resource_dir.join("resources").join(AGENT_BUNDLE_DIR));

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("resources").join(AGENT_BUNDLE_DIR));
        candidates.push(
            cwd.join("src-tauri")
                .join("resources")
                .join(AGENT_BUNDLE_DIR),
        );
    }

    if let Ok(win_root) = std::env::var("INTERMEDIARY_WIN_PATH") {
        if !win_root.trim().is_empty() {
            candidates.push(
                PathBuf::from(win_root)
                    .join("src-tauri")
                    .join("resources")
                    .join(AGENT_BUNDLE_DIR),
            );
        }
    }

    for candidate in candidates {
        tried.push(candidate.clone());
        if !candidate.is_dir() {
            continue;
        }
        if bundle_has_core_files(&candidate) {
            return Ok(candidate);
        }
    }

    let attempted = tried
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    Err(format!(
        "Agent bundle resources missing required files for this platform. Tried: {attempted}"
    ))
}

fn bundle_has_core_files(bundle_dir: &Path) -> bool {
    if !bundle_dir.join(AGENT_VERSION_FILE).is_file() {
        return false;
    }
    if requires_wsl_binary() && !bundle_dir.join(WSL_AGENT_BINARY_FILE).is_file() {
        return false;
    }
    true
}

fn requires_wsl_binary() -> bool {
    cfg!(target_os = "windows")
}
