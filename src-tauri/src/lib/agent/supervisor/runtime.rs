// Path: src-tauri/src/lib/agent/supervisor/runtime.rs
// Description: Supervisor runtime path, port, and installed-bundle preference helpers

use tauri::{AppHandle, Manager};

pub(super) fn resolve_wsl_port(host_port: u16, requires_wsl: bool) -> Result<u16, String> {
    if !requires_wsl {
        return Ok(host_port.saturating_add(1));
    }

    host_port
        .checked_add(1)
        .ok_or_else(|| "Agent port 65535 cannot reserve WSL backend port".to_string())
}

pub(super) fn resolve_expected_dirs(app: &AppHandle) -> Result<(String, String), String> {
    let app_local_data = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "Failed to resolve app local data directory".to_string())?;
    let agent_dir = app_local_data.join("agent");
    let log_dir = app_local_data.join("logs");
    Ok((
        agent_dir.display().to_string(),
        log_dir.display().to_string(),
    ))
}

pub(super) fn should_prefer_installed_bundle(host_listening: bool, wsl_listening: bool) -> bool {
    if cfg!(debug_assertions) {
        return false;
    }

    host_listening || wsl_listening
}

pub(super) fn should_adopt_running_runtime(
    replace_runtime: bool,
    host_runtime_matches: bool,
    host_origin_compat_ok: bool,
    requires_wsl: bool,
    wsl_runtime_matches: bool,
    managed_owner_required: bool,
) -> bool {
    !replace_runtime
        && host_runtime_matches
        && host_origin_compat_ok
        && (!requires_wsl || (wsl_runtime_matches && !managed_owner_required))
}

#[cfg(test)]
mod tests {
    use super::{resolve_wsl_port, should_adopt_running_runtime, should_prefer_installed_bundle};

    #[test]
    fn resolve_wsl_port_for_wsl_repos_uses_next_port() {
        assert_eq!(resolve_wsl_port(3141, true).expect("port"), 3142);
    }

    #[test]
    fn resolve_wsl_port_for_windows_only_allows_max_host_port() {
        assert_eq!(resolve_wsl_port(u16::MAX, false).expect("port"), u16::MAX);
    }

    #[test]
    fn resolve_wsl_port_for_wsl_repos_rejects_u16_overflow() {
        let error = resolve_wsl_port(u16::MAX, true).expect_err("expected overflow");
        assert!(
            error.contains("cannot reserve WSL backend port"),
            "unexpected error: {error}"
        );
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn installed_bundle_is_preferred_when_host_or_wsl_is_alive() {
        assert!(should_prefer_installed_bundle(true, false));
        assert!(should_prefer_installed_bundle(false, true));
        assert!(should_prefer_installed_bundle(true, true));
        assert!(!should_prefer_installed_bundle(false, false));
    }

    #[cfg(debug_assertions)]
    #[test]
    fn installed_bundle_is_not_preferred_in_debug_builds() {
        assert!(!should_prefer_installed_bundle(true, false));
        assert!(!should_prefer_installed_bundle(false, true));
        assert!(!should_prefer_installed_bundle(true, true));
        assert!(!should_prefer_installed_bundle(false, false));
    }

    #[test]
    fn managed_mode_disables_wsl_already_running_fast_path() {
        assert!(!should_adopt_running_runtime(
            false, true, true, true, true, true
        ));
    }

    #[test]
    fn current_wsl_runtime_keeps_non_managed_fast_path() {
        assert!(should_adopt_running_runtime(
            false, true, true, true, true, false
        ));
    }

    #[test]
    fn origin_mismatch_disables_already_running_fast_path() {
        assert!(!should_adopt_running_runtime(
            false, true, false, true, true, false
        ));
    }

    #[test]
    fn runtime_replacement_disables_wsl_already_running_fast_path() {
        assert!(!should_adopt_running_runtime(
            true, true, true, true, true, false
        ));
    }

    #[test]
    fn current_host_only_runtime_keeps_already_running_fast_path() {
        assert!(should_adopt_running_runtime(
            false, true, true, false, false, false
        ));
    }

    #[test]
    fn runtime_replacement_disables_host_only_already_running_fast_path() {
        assert!(!should_adopt_running_runtime(
            true, true, true, false, false, false
        ));
    }

    #[test]
    fn stale_host_process_disables_already_running_fast_path() {
        assert!(!should_adopt_running_runtime(
            false, false, true, false, false, false
        ));
    }
}
