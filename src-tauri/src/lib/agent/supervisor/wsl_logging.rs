// Path: src-tauri/src/lib/agent/supervisor/wsl_logging.rs
// Description: Structured WSL backend ownership and authentication lifecycle logging

use super::wsl_mode::{WslBackendMode, WslBackendOwner};
use crate::agent::wsl_process_control::{format_wsl_target, WslLaunchTarget};
use crate::obs::logging;

pub(super) fn log_wsl_owner_detection(
    backend_mode: WslBackendMode,
    owner: WslBackendOwner,
    wsl_port: u16,
    target: &WslLaunchTarget,
) {
    logging::log(
        "info",
        "agent",
        "wsl_owner_detected",
        &format!(
            "mode={} owner={} port={wsl_port} {}",
            backend_mode.log_key(),
            owner.log_key(),
            format_wsl_target(target)
        ),
    );
}

pub(super) fn log_wsl_owner_mismatch(
    backend_mode: WslBackendMode,
    wsl_port: u16,
    target: &WslLaunchTarget,
) {
    logging::log(
        "warn",
        "agent",
        "wsl_external_unmanaged_auth_failed",
        &format!(
            "mode={} owner=external_unmanaged port={wsl_port} {}",
            backend_mode.log_key(),
            format_wsl_target(target)
        ),
    );
}

pub(super) fn log_wsl_external_auth_failure(
    mode: &str,
    owner: WslBackendOwner,
    wsl_port: u16,
    target: &WslLaunchTarget,
) {
    logging::log(
        "warn",
        "agent",
        "wsl_external_unmanaged_auth_failed",
        &format!(
            "mode={mode} owner={} port={wsl_port} {}",
            owner.log_key(),
            format_wsl_target(target)
        ),
    );
}
