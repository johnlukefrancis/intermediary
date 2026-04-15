// Path: src-tauri/src/lib/agent/install_runtime.rs
// Description: Agent bundle install/runtime helpers for version checks, file copying, and stale-host cleanup

use super::host_process_control::{terminate_host_agent_process, HostTerminateOutcome};
use super::install::AgentBundlePaths;
use super::install_host_binary::{copy_host_binary, resolve_host_binary_source};
use serde::Deserialize;
use std::fs;
use std::io::ErrorKind;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;
use std::time::Duration;

const HOST_TERMINATE_GRACE: Duration = Duration::from_secs(5);
const HOST_TERMINATE_POLL: Duration = Duration::from_millis(50);

#[derive(Debug, Deserialize)]
struct AgentBundleVersion {
    version: String,
}

pub(super) fn installed_bundle_matches(
    bundle_dir: &Path,
    installed_dir: &Path,
    version_file: &str,
    wsl_agent_binary_file: &str,
    host_agent_binary_file: &str,
    requires_wsl_binary: bool,
) -> Result<bool, String> {
    if !installed_dir.is_dir() {
        return Ok(false);
    }

    let bundle_version = bundle_dir.join(version_file);
    if !bundle_version.is_file() {
        return Err(format!(
            "Agent bundle missing required file: {version_file}"
        ));
    }
    if requires_wsl_binary {
        let bundle_wsl = bundle_dir.join(wsl_agent_binary_file);
        if !bundle_wsl.is_file() {
            return Err(format!(
                "Agent bundle missing required file: {wsl_agent_binary_file}"
            ));
        }
    }

    let installed_version = installed_dir.join(version_file);
    if !installed_version.is_file() {
        return Ok(false);
    }

    if !files_equal(&bundle_version, &installed_version)? {
        return Ok(false);
    }
    if requires_wsl_binary {
        let bundle_wsl = bundle_dir.join(wsl_agent_binary_file);
        let installed_wsl = installed_dir.join(wsl_agent_binary_file);
        if !installed_wsl.is_file() {
            return Ok(false);
        }
        if !files_equal(&bundle_wsl, &installed_wsl)? {
            return Ok(false);
        }
    }

    let (host_source, _) =
        resolve_host_binary_source(bundle_dir, installed_dir, host_agent_binary_file);
    let Some(host_source) = host_source else {
        return Ok(installed_dir.join(host_agent_binary_file).is_file());
    };
    let installed_host = installed_dir.join(host_agent_binary_file);
    if !installed_host.is_file() {
        return Ok(false);
    }

    files_equal(&host_source, &installed_host)
}

pub(super) fn install_bundle(
    bundle_dir: &Path,
    agent_dir_host: &Path,
    version_file: &str,
    wsl_agent_binary_file: &str,
    host_agent_binary_file: &str,
    requires_wsl_binary: bool,
) -> Result<(), String> {
    let temp_dir = agent_dir_host.with_extension("tmp");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)
            .map_err(|err| format!("Failed to clear temp agent bundle: {err}"))?;
    }

    fs::create_dir_all(&temp_dir)
        .map_err(|err| format!("Failed to create agent bundle temp dir: {err}"))?;

    if requires_wsl_binary {
        copy_required(bundle_dir, &temp_dir, wsl_agent_binary_file)?;
    }
    copy_host_binary(
        bundle_dir,
        &temp_dir,
        agent_dir_host,
        host_agent_binary_file,
    )?;
    copy_required(bundle_dir, &temp_dir, version_file)?;

    remove_existing_agent_dir(agent_dir_host, host_agent_binary_file)?;

    fs::rename(&temp_dir, agent_dir_host)
        .map_err(|err| format!("Failed to install agent bundle: {err}"))?;

    Ok(())
}

pub(super) fn read_installed_version(path: &Path) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    read_version(path).ok()
}

pub(super) fn read_version(path: &Path) -> Result<String, String> {
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

pub(super) fn build_bundle_paths(
    agent_dir_host: PathBuf,
    log_dir_host: PathBuf,
    version: String,
    wsl_agent_binary_file: &str,
    host_agent_binary_file: &str,
    requires_wsl_binary: bool,
) -> Result<AgentBundlePaths, String> {
    if let Err(err) = fs::create_dir_all(&log_dir_host) {
        return Err(format!("Failed to create log directory: {err}"));
    }

    let wsl_agent_binary_host = if requires_wsl_binary {
        Some(agent_dir_host.join(wsl_agent_binary_file))
    } else {
        None
    };
    let host_agent_binary_host = agent_dir_host.join(host_agent_binary_file);
    ensure_host_agent_permissions(&host_agent_binary_host)?;

    Ok(AgentBundlePaths {
        host_agent_binary_host,
        agent_dir_host,
        log_dir_host,
        wsl_agent_binary_host,
        version,
    })
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

fn copy_required(bundle_dir: &Path, target_dir: &Path, file_name: &str) -> Result<(), String> {
    let source = bundle_dir.join(file_name);
    if !source.is_file() {
        return Err(format!("Agent bundle missing required file: {file_name}"));
    }
    let dest = target_dir.join(file_name);
    fs::copy(&source, &dest).map_err(|err| format!("Failed to copy {file_name}: {err}"))?;
    Ok(())
}

fn remove_existing_agent_dir(
    agent_dir_host: &Path,
    host_agent_binary_file: &str,
) -> Result<(), String> {
    if !agent_dir_host.exists() {
        return Ok(());
    }

    match fs::remove_dir_all(agent_dir_host) {
        Ok(()) => Ok(()),
        Err(err) => remediate_locked_agent_dir(agent_dir_host, host_agent_binary_file, err),
    }
}

fn remediate_locked_agent_dir(
    agent_dir_host: &Path,
    host_agent_binary_file: &str,
    initial_error: std::io::Error,
) -> Result<(), String> {
    if !cfg!(target_os = "windows") || initial_error.kind() != ErrorKind::PermissionDenied {
        return Err(format!(
            "Failed to remove old agent bundle: {initial_error}"
        ));
    }

    let installed_host_binary = agent_dir_host.join(host_agent_binary_file);
    match terminate_host_agent_process(
        &installed_host_binary,
        HOST_TERMINATE_GRACE,
        HOST_TERMINATE_POLL,
    ) {
        Ok(HostTerminateOutcome::Terminated) => fs::remove_dir_all(agent_dir_host).map_err(|err| {
            format!(
                "Failed to remove old agent bundle after terminating stale host agent ({}): {err}",
                installed_host_binary.display()
            )
        }),
        Ok(HostTerminateOutcome::NoMatch) => {
            Err(format!("Failed to remove old agent bundle: {initial_error}"))
        }
        Err(err) => Err(format!(
            "Failed to remove old agent bundle because the installed host agent could not be retired ({}): {err}",
            installed_host_binary.display()
        )),
    }
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
