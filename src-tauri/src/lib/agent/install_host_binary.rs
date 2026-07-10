// Path: src-tauri/src/lib/agent/install_host_binary.rs
// Description: Resolve and copy the correct host-agent binary into an install bundle staging directory

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;

const AGENT_BUNDLE_DIR: &str = "agent_bundle";

pub(super) fn copy_host_binary(
    bundle_dir: &Path,
    target_dir: &Path,
    installed_agent_dir: &Path,
    host_agent_binary_file: &str,
) -> Result<(), String> {
    let (source, searched) =
        resolve_host_binary_source(bundle_dir, installed_agent_dir, host_agent_binary_file);
    let source = source.ok_or_else(|| {
        format!(
            "Agent bundle missing required file: {host_agent_binary_file}. Searched: {}",
            searched.join(", ")
        )
    })?;

    let dest = target_dir.join(host_agent_binary_file);
    fs::copy(&source, &dest).map_err(|err| {
        format!(
            "Failed to copy {host_agent_binary_file} from {}: {err}",
            source.display()
        )
    })?;
    Ok(())
}

pub(super) fn resolve_host_binary_source(
    bundle_dir: &Path,
    installed_agent_dir: &Path,
    host_agent_binary_file: &str,
) -> (Option<PathBuf>, Vec<String>) {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(cwd) = std::env::current_dir() {
        push_host_binary_candidates(&mut candidates, &cwd, host_agent_binary_file);
        push_host_binary_candidates(&mut candidates, &cwd.join(".."), host_agent_binary_file);
    }
    if let Ok(win_root) = std::env::var("INTERMEDIARY_WIN_PATH") {
        if !win_root.trim().is_empty() {
            let root = PathBuf::from(win_root);
            push_host_binary_candidates(&mut candidates, &root, host_agent_binary_file);
            candidates.push(
                root.join("src-tauri")
                    .join("resources")
                    .join(AGENT_BUNDLE_DIR)
                    .join(host_agent_binary_file),
            );
        }
    }
    candidates.push(bundle_dir.join(host_agent_binary_file));
    candidates.push(installed_agent_dir.join(host_agent_binary_file));

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

fn push_host_binary_candidates(
    candidates: &mut Vec<PathBuf>,
    root: &Path,
    host_agent_binary_file: &str,
) {
    if cfg!(debug_assertions) {
        candidates.push(
            root.join("target")
                .join("debug")
                .join(host_agent_binary_file),
        );
        candidates.push(
            root.join("target")
                .join("release")
                .join(host_agent_binary_file),
        );
        return;
    }

    candidates.push(
        root.join("target")
            .join("release")
            .join(host_agent_binary_file),
    );
    candidates.push(
        root.join("target")
            .join("debug")
            .join(host_agent_binary_file),
    );
}

#[cfg(unix)]
pub(super) fn ensure_host_agent_permissions(host_agent_binary: &Path) -> Result<(), String> {
    let permissions = fs::Permissions::from_mode(0o755);
    fs::set_permissions(host_agent_binary, permissions).map_err(|err| {
        format!(
            "Failed to set executable permissions on host agent binary ({}): {err}",
            host_agent_binary.display()
        )
    })?;

    #[cfg(target_os = "macos")]
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

    Ok(())
}

#[cfg(not(unix))]
pub(super) fn ensure_host_agent_permissions(_host_agent_binary: &Path) -> Result<(), String> {
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
