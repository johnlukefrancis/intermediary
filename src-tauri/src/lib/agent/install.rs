// Path: src-tauri/src/lib/agent/install.rs
// Description: Install bundled agent runtimes into app local data with platform-specific requirements

use serde::Deserialize;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;

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

    let agent_dir_host = app_local_data.join(AGENT_INSTALL_DIR);
    let installed_version_path = agent_dir_host.join(AGENT_VERSION_FILE);
    let installed_version = read_installed_version(&installed_version_path);

    let should_install = if installed_version.as_deref() != Some(version.as_str()) {
        true
    } else {
        !installed_bundle_matches(&bundle_dir, &agent_dir_host)?
    };

    if should_install {
        install_bundle(&bundle_dir, &agent_dir_host)?;
    }

    build_bundle_paths(agent_dir_host, app_local_data.join("logs"), version)
}

fn installed_bundle_matches(bundle_dir: &Path, installed_dir: &Path) -> Result<bool, String> {
    if !installed_dir.is_dir() {
        return Ok(false);
    }

    let bundle_version = bundle_dir.join(AGENT_VERSION_FILE);
    if !bundle_version.is_file() {
        return Err(format!(
            "Agent bundle missing required file: {AGENT_VERSION_FILE}"
        ));
    }
    if requires_wsl_binary() {
        let bundle_wsl = bundle_dir.join(WSL_AGENT_BINARY_FILE);
        if !bundle_wsl.is_file() {
            return Err(format!(
                "Agent bundle missing required file: {WSL_AGENT_BINARY_FILE}"
            ));
        }
    }

    let installed_version = installed_dir.join(AGENT_VERSION_FILE);
    if !installed_version.is_file() {
        return Ok(false);
    }

    if !files_equal(&bundle_version, &installed_version)? {
        return Ok(false);
    }
    if requires_wsl_binary() {
        let bundle_wsl = bundle_dir.join(WSL_AGENT_BINARY_FILE);
        let installed_wsl = installed_dir.join(WSL_AGENT_BINARY_FILE);
        if !installed_wsl.is_file() {
            return Ok(false);
        }
        if !files_equal(&bundle_wsl, &installed_wsl)? {
            return Ok(false);
        }
    }

    let (host_source, _) = resolve_host_binary_source(bundle_dir, installed_dir);
    let Some(host_source) = host_source else {
        return Ok(installed_dir.join(HOST_AGENT_BINARY_FILE).is_file());
    };
    let installed_host = installed_dir.join(HOST_AGENT_BINARY_FILE);
    if !installed_host.is_file() {
        return Ok(false);
    }

    files_equal(&host_source, &installed_host)
}

fn files_equal(left: &Path, right: &Path) -> Result<bool, String> {
    let left_meta =
        fs::metadata(left).map_err(|err| format!("Failed to stat {}: {err}", left.display()))?;
    let right_meta =
        fs::metadata(right).map_err(|err| format!("Failed to stat {}: {err}", right.display()))?;

    if left_meta.len() != right_meta.len() {
        return Ok(false);
    }

    let left_bytes =
        fs::read(left).map_err(|err| format!("Failed to read {}: {err}", left.display()))?;
    let right_bytes =
        fs::read(right).map_err(|err| format!("Failed to read {}: {err}", right.display()))?;

    Ok(left_bytes == right_bytes)
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

    build_bundle_paths(agent_dir_host, app_local_data.join("logs"), version)
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

fn install_bundle(bundle_dir: &Path, agent_dir_host: &Path) -> Result<(), String> {
    let temp_dir = agent_dir_host.with_extension("tmp");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)
            .map_err(|err| format!("Failed to clear temp agent bundle: {err}"))?;
    }

    fs::create_dir_all(&temp_dir)
        .map_err(|err| format!("Failed to create agent bundle temp dir: {err}"))?;

    if requires_wsl_binary() {
        copy_required(bundle_dir, &temp_dir, WSL_AGENT_BINARY_FILE)?;
    }
    copy_host_binary(bundle_dir, &temp_dir, agent_dir_host)?;
    copy_required(bundle_dir, &temp_dir, AGENT_VERSION_FILE)?;

    if agent_dir_host.exists() {
        fs::remove_dir_all(agent_dir_host)
            .map_err(|err| format!("Failed to remove old agent bundle: {err}"))?;
    }

    fs::rename(&temp_dir, agent_dir_host)
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

fn copy_host_binary(
    bundle_dir: &Path,
    target_dir: &Path,
    installed_agent_dir: &Path,
) -> Result<(), String> {
    let (source, searched) = resolve_host_binary_source(bundle_dir, installed_agent_dir);
    let source = source.ok_or_else(|| {
        format!(
            "Agent bundle missing required file: {HOST_AGENT_BINARY_FILE}. Searched: {}",
            searched.join(", ")
        )
    })?;

    let dest = target_dir.join(HOST_AGENT_BINARY_FILE);
    fs::copy(&source, &dest).map_err(|err| {
        format!(
            "Failed to copy {HOST_AGENT_BINARY_FILE} from {}: {err}",
            source.display()
        )
    })?;
    Ok(())
}

fn resolve_host_binary_source(
    bundle_dir: &Path,
    installed_agent_dir: &Path,
) -> (Option<PathBuf>, Vec<String>) {
    let mut candidates: Vec<PathBuf> = Vec::new();

    candidates.push(bundle_dir.join(HOST_AGENT_BINARY_FILE));
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(
            cwd.join("target")
                .join("release")
                .join(HOST_AGENT_BINARY_FILE),
        );
        candidates.push(
            cwd.join("target")
                .join("debug")
                .join(HOST_AGENT_BINARY_FILE),
        );
        candidates.push(
            cwd.join("..")
                .join("target")
                .join("release")
                .join(HOST_AGENT_BINARY_FILE),
        );
        candidates.push(
            cwd.join("..")
                .join("target")
                .join("debug")
                .join(HOST_AGENT_BINARY_FILE),
        );
    }
    if let Ok(win_root) = std::env::var("INTERMEDIARY_WIN_PATH") {
        if !win_root.trim().is_empty() {
            let root = PathBuf::from(win_root);
            candidates.push(
                root.join("target")
                    .join("release")
                    .join(HOST_AGENT_BINARY_FILE),
            );
            candidates.push(
                root.join("target")
                    .join("debug")
                    .join(HOST_AGENT_BINARY_FILE),
            );
            candidates.push(
                root.join("src-tauri")
                    .join("resources")
                    .join(AGENT_BUNDLE_DIR)
                    .join(HOST_AGENT_BINARY_FILE),
            );
        }
    }
    candidates.push(installed_agent_dir.join(HOST_AGENT_BINARY_FILE));

    let searched = candidates
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();

    for candidate in candidates {
        if candidate.is_file() {
            return (Some(candidate), searched);
        }
    }
    (None, searched)
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

fn build_bundle_paths(
    agent_dir_host: PathBuf,
    log_dir_host: PathBuf,
    version: String,
) -> Result<AgentBundlePaths, String> {
    if let Err(err) = fs::create_dir_all(&log_dir_host) {
        return Err(format!("Failed to create log directory: {err}"));
    }

    let wsl_agent_binary_host = if requires_wsl_binary() {
        Some(agent_dir_host.join(WSL_AGENT_BINARY_FILE))
    } else {
        None
    };
    let host_agent_binary_host = agent_dir_host.join(HOST_AGENT_BINARY_FILE);
    ensure_host_agent_permissions(&host_agent_binary_host)?;

    Ok(AgentBundlePaths {
        host_agent_binary_host,
        agent_dir_host,
        log_dir_host,
        wsl_agent_binary_host,
        version,
    })
}

fn requires_wsl_binary() -> bool {
    cfg!(target_os = "windows")
}

#[cfg(unix)]
fn ensure_host_agent_permissions(host_agent_binary: &Path) -> Result<(), String> {
    let permissions = fs::Permissions::from_mode(0o755);
    fs::set_permissions(host_agent_binary, permissions).map_err(|err| {
        format!(
            "Failed to set executable permissions on host agent binary ({}): {err}",
            host_agent_binary.display()
        )
    })?;

    #[cfg(target_os = "macos")]
    {
        if let Err(err) = clear_macos_quarantine(host_agent_binary) {
            crate::obs::logging::log(
                "warn",
                "agent",
                "clear_quarantine_failed",
                &format!(
                    "Could not clear com.apple.quarantine on {}: {err}",
                    host_agent_binary.display()
                ),
            );
        }
    }

    Ok(())
}

#[cfg(not(unix))]
fn ensure_host_agent_permissions(_host_agent_binary: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn clear_macos_quarantine(host_agent_binary: &Path) -> Result<(), String> {
    let output = Command::new("xattr")
        .arg("-d")
        .arg("com.apple.quarantine")
        .arg(host_agent_binary)
        .output()
        .map_err(|err| format!("Failed to execute xattr: {err}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("No such xattr") {
        return Ok(());
    }

    Err(format!(
        "xattr exited with status {}. stderr: {}",
        output.status,
        stderr.trim()
    ))
}
