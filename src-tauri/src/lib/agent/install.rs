// Path: src-tauri/src/lib/agent/install.rs
// Description: Install bundled agent runtimes into app local data with platform-specific requirements

use super::bundle_resources::{read_version, resolve_bundle_dir};
#[cfg(not(target_os = "macos"))]
use super::install_runtime::{
    build_bundle_paths, install_bundle, installed_bundle_matches, read_installed_version,
};
use std::path::{Path, PathBuf};

pub(super) const AGENT_BUNDLE_DIR: &str = "agent_bundle";
#[cfg(any(not(target_os = "macos"), test))]
const AGENT_INSTALL_DIR: &str = "agent";
pub(super) const WSL_AGENT_BINARY_FILE: &str = "im_agent";
#[cfg(target_os = "windows")]
const HOST_AGENT_BINARY_FILE: &str = "im_host_agent.exe";
#[cfg(not(target_os = "windows"))]
const HOST_AGENT_BINARY_FILE: &str = "im_host_agent";
pub(super) const AGENT_VERSION_FILE: &str = "version.json";

#[derive(Debug, Clone)]
pub struct AgentBundlePaths {
    pub agent_dir_host: PathBuf,
    pub log_dir_host: PathBuf,
    pub host_agent_binary_host: PathBuf,
    pub host_agent_sha256: String,
    pub wsl_agent_binary_host: Option<PathBuf>,
    pub wsl_agent_sha256: Option<String>,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentBundleInstallState {
    Current,
    Installed,
}

#[derive(Debug, Clone)]
pub struct AgentBundleResolution {
    pub bundle: AgentBundlePaths,
    pub install_state: AgentBundleInstallState,
}

#[cfg(not(target_os = "macos"))]
pub fn ensure_agent_bundle(
    resource_dir: &Path,
    app_local_data: &Path,
) -> Result<AgentBundleResolution, String> {
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

    let install_state = if should_install {
        install_bundle(
            &bundle_dir,
            &agent_dir_host,
            AGENT_VERSION_FILE,
            WSL_AGENT_BINARY_FILE,
            HOST_AGENT_BINARY_FILE,
            requires_wsl_binary(),
        )?;
        AgentBundleInstallState::Installed
    } else {
        AgentBundleInstallState::Current
    };

    let bundle = build_bundle_paths(
        agent_dir_host,
        app_local_data.join("logs"),
        version,
        WSL_AGENT_BINARY_FILE,
        HOST_AGENT_BINARY_FILE,
        requires_wsl_binary(),
    )?;

    Ok(AgentBundleResolution {
        bundle,
        install_state,
    })
}

#[cfg(not(target_os = "macos"))]
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
    _prefer_installed: bool,
) -> Result<AgentBundleResolution, String> {
    #[cfg(target_os = "macos")]
    {
        let bundle_dir = resolve_bundle_dir(resource_dir)?;
        let binary = bundle_dir.join(HOST_AGENT_BINARY_FILE);
        let version = read_version(&bundle_dir.join(AGENT_VERSION_FILE))?;
        let log_dir_host = app_local_data.join("logs");
        std::fs::create_dir_all(&log_dir_host)
            .map_err(|err| format!("Failed to create agent logs: {err}"))?;
        return Ok(AgentBundleResolution {
            bundle: AgentBundlePaths {
                host_agent_sha256: super::runtime_identity::executable_sha256(&binary)?,
                host_agent_binary_host: binary,
                agent_dir_host: bundle_dir,
                log_dir_host,
                wsl_agent_binary_host: None,
                wsl_agent_sha256: None,
                version,
            },
            install_state: AgentBundleInstallState::Current,
        });
    }
    #[cfg(not(target_os = "macos"))]
    {
        if _prefer_installed {
            if let Some(bundle) =
                resolve_current_installed_agent_bundle(resource_dir, app_local_data)?
            {
                return Ok(AgentBundleResolution {
                    bundle,
                    install_state: AgentBundleInstallState::Current,
                });
            }
        }

        ensure_agent_bundle(resource_dir, app_local_data)
    }
}

#[cfg(not(target_os = "macos"))]
fn resolve_current_installed_agent_bundle(
    resource_dir: &Path,
    app_local_data: &Path,
) -> Result<Option<AgentBundlePaths>, String> {
    let Ok(bundle) = resolve_installed_agent_bundle(app_local_data) else {
        return Ok(None);
    };

    let bundle_dir = resolve_bundle_dir(resource_dir)?;
    if installed_bundle_matches(
        &bundle_dir,
        &bundle.agent_dir_host,
        AGENT_VERSION_FILE,
        WSL_AGENT_BINARY_FILE,
        HOST_AGENT_BINARY_FILE,
        requires_wsl_binary(),
    )? {
        return Ok(Some(bundle));
    }

    Ok(None)
}

pub(super) fn requires_wsl_binary() -> bool {
    cfg!(target_os = "windows")
}

#[cfg(test)]
#[path = "install_tests.rs"]
mod tests;
