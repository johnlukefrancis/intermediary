// Path: src-tauri/src/lib/agent/supervisor/wsl.rs
// Description: The ensure-running decision for the WSL backend: ownership detection, adoption, remediation

use super::wsl_logging::{
    log_wsl_external_auth_failure, log_wsl_owner_detection, log_wsl_owner_mismatch,
};
use super::wsl_mode::{
    backend_mode_allows_owner, resolve_wsl_backend_mode, WslBackendMode, WslBackendOwner,
};
use super::wsl_spawn::spawn_wsl_supervised;
use super::{AgentSupervisor, EnsureProcessResult};
use crate::agent::install::AgentBundlePaths;
use crate::agent::supervisor::state::ProcessKind;
use crate::agent::wsl_process_control::{build_wsl_launch_target, format_wsl_target};
use crate::agent::AgentWebSocketAuth;
use crate::obs::logging;

impl AgentSupervisor {
    pub(super) async fn ensure_wsl_running(
        &self,
        bundle: &AgentBundlePaths,
        distro: Option<&str>,
        wsl_port: u16,
        auth: &AgentWebSocketAuth,
        force: bool,
    ) -> Result<EnsureProcessResult, String> {
        let target = build_wsl_launch_target(bundle, distro)?;
        let (backend_mode, invalid_mode_raw) = resolve_wsl_backend_mode();
        self.set_wsl_launch_target(None)?;
        // Clear ownership records at the start of every pass; they are re-committed only once
        // ownership is confirmed reclaimable (record_owned_wsl_backend). A foreign/ExternalUnmanaged
        // occupant therefore never leaves a durable handle behind — so app-exit teardown can never
        // terminate a foreign distro (ADR-013 boundary).
        self.set_last_wsl_backend(None)?;
        if let Some(invalid) = invalid_mode_raw {
            logging::log(
                "warn",
                "agent",
                "wsl_mode_invalid",
                &format!(
                    "invalid={invalid} env=INTERMEDIARY_WSL_BACKEND_MODE defaulted=auto {}",
                    format_wsl_target(&target)
                ),
            );
        }

        if self.probe_listening(wsl_port).await? {
            let owner = self.detect_wsl_backend_owner(&target, wsl_port).await?;
            if owner == WslBackendOwner::ExternalUnmanaged {
                self.reconcile_recorded_child(ProcessKind::Wsl, "external_unmanaged_detected")
                    .await?;
            }
            log_wsl_owner_detection(backend_mode, owner, wsl_port, &target);

            if !backend_mode_allows_owner(backend_mode, owner) {
                let Some(message) = AgentSupervisor::managed_mode_error_for_external_occupant(
                    backend_mode,
                    wsl_port,
                    &target,
                ) else {
                    return Err(
                        "Managed WSL ownership policy rejected a listener without an actionable error"
                            .to_string(),
                    );
                };
                log_wsl_owner_mismatch(backend_mode, wsl_port, &target);
                return Err(message);
            }

            // Reclaimable = a backend we may terminate: one of our own agents (installed or
            // same-port Intermediary), outside external mode.
            let reclaimable_owner = !matches!(owner, WslBackendOwner::ExternalUnmanaged)
                && !matches!(backend_mode, WslBackendMode::External);

            let mut remediated_current_listener = false;
            let identity = self
                .probe_websocket_identity(wsl_port, &auth.wsl_ws_token)
                .await?;
            if identity.authenticated {
                // A forced restart must tear a healthy backend down and respawn it — otherwise
                // "Restart Agent" silently no-ops. Managed mode also remediates a same-port owner
                // to converge on the installed backend.
                let force_respawn = force && reclaimable_owner;
                let runtime_matches = bundle
                    .wsl_agent_sha256
                    .as_deref()
                    .is_some_and(|expected| identity.matches_runtime(expected));
                let identity_respawn =
                    should_respawn_for_runtime_identity(reclaimable_owner, runtime_matches);
                let managed_owner_remediation = owner == WslBackendOwner::SamePortIntermediary
                    && matches!(backend_mode, WslBackendMode::Managed);
                if force_respawn || identity_respawn || managed_owner_remediation {
                    self.record_owned_wsl_backend(&target, wsl_port)?;
                    let reason = if force_respawn {
                        "force_restart"
                    } else if identity_respawn {
                        "runtime_identity_mismatch"
                    } else {
                        "managed_owner_mismatch"
                    };
                    self.remediate_stale_wsl_port(wsl_port, &target, owner, reason)
                        .await?;
                    remediated_current_listener = true;
                } else {
                    // Adopt the healthy running backend as ours to manage, recording the kill
                    // target so stop/exit can terminate it. Skipped in external mode.
                    //
                    // An adopted agent was spawned by some earlier process, so this
                    // supervisor holds no `Child` and therefore no stdin pipe: its
                    // shutdown owners are the `shutdown` command and SIGTERM only,
                    // never EOF (`im_agent::server::stdin_eof`). Logged, because it
                    // is the one route where losing this supervisor does not by
                    // itself ask the agent to drain.
                    if reclaimable_owner {
                        self.record_owned_wsl_backend(&target, wsl_port)?;
                    }
                    logging::log(
                        "info",
                        "agent",
                        "wsl_adopt",
                        &format!(
                            "kind=wsl phase=adopt outcome=adopted port={wsl_port} stdin_pipe=none reclaimable={reclaimable_owner} {}",
                            format_wsl_target(&target)
                        ),
                    );
                    return Ok(EnsureProcessResult::AlreadyRunning);
                }
            }

            if !remediated_current_listener {
                // Check ownership BEFORE recording it: a foreign occupant must return here with no
                // durable handle recorded, so exit teardown never touches its distro.
                if let Some(error) =
                    self.wsl_auth_failure_error(backend_mode, owner, wsl_port, &target)
                {
                    return Err(error);
                }

                self.record_owned_wsl_backend(&target, wsl_port)?;
                self.remediate_stale_wsl_port(wsl_port, &target, owner, "auth_probe_failed")
                    .await?;
            }
        } else if matches!(backend_mode, WslBackendMode::External) {
            return Err(format!(
                "WSL backend mode=external requires an externally managed backend listening on port {wsl_port} ({})",
                format_wsl_target(&target)
            ));
        }

        // Reached only when spawning our own backend (port free, or occupant remediated). Commit
        // ownership so stop/exit can reclaim the process we are about to launch.
        self.record_owned_wsl_backend(&target, wsl_port)?;
        self.reconcile_recorded_child(ProcessKind::Wsl, "port_probe_failed")
            .await?;
        if !force && self.is_in_backoff(ProcessKind::Wsl)? {
            return Ok(EnsureProcessResult::Backoff);
        }

        spawn_wsl_supervised(self, bundle, &target, wsl_port, &auth.wsl_ws_token).await
    }

    async fn detect_wsl_backend_owner(
        &self,
        target: &crate::agent::wsl_process_control::WslLaunchTarget,
        wsl_port: u16,
    ) -> Result<WslBackendOwner, String> {
        let installed_pid_count = self.detect_installed_wsl_pid_count(target).await?;
        if installed_pid_count > 0 {
            return Ok(WslBackendOwner::InstalledManaged);
        }

        let same_port_pid_count = self
            .detect_intermediary_wsl_pid_count_by_port(target, wsl_port)
            .await?;
        if same_port_pid_count > 0 {
            return Ok(WslBackendOwner::SamePortIntermediary);
        }

        Ok(WslBackendOwner::ExternalUnmanaged)
    }

    fn wsl_auth_failure_error(
        &self,
        backend_mode: WslBackendMode,
        owner: WslBackendOwner,
        wsl_port: u16,
        target: &crate::agent::wsl_process_control::WslLaunchTarget,
    ) -> Option<String> {
        match (backend_mode, owner) {
            (WslBackendMode::External, _) => {
                log_wsl_external_auth_failure("external", owner, wsl_port, target);
                Some(format!(
                    "WSL backend auth failed on port {wsl_port} while mode=external (owner={}, {}). Ensure the external backend token matches app websocket auth state.",
                    owner.log_key(),
                    format_wsl_target(target)
                ))
            }
            (WslBackendMode::Auto, WslBackendOwner::ExternalUnmanaged) => {
                log_wsl_external_auth_failure("auto", owner, wsl_port, target);
                Some(format!(
                    "WSL backend port {wsl_port} is occupied by an external process that rejected the current websocket token ({}) and will not be terminated in mode=auto.",
                    format_wsl_target(target)
                ))
            }
            (WslBackendMode::Managed, WslBackendOwner::ExternalUnmanaged) => {
                log_wsl_external_auth_failure("managed", owner, wsl_port, target);
                AgentSupervisor::managed_mode_error_for_external_occupant(
                    backend_mode,
                    wsl_port,
                    target,
                )
            }
            (_, WslBackendOwner::InstalledManaged | WslBackendOwner::SamePortIntermediary) => None,
        }
    }
}

fn should_respawn_for_runtime_identity(reclaimable_owner: bool, runtime_matches: bool) -> bool {
    reclaimable_owner && !runtime_matches
}

#[cfg(test)]
mod tests {
    use super::should_respawn_for_runtime_identity;

    #[test]
    fn stale_reclaimable_backend_is_replaced() {
        assert!(should_respawn_for_runtime_identity(true, false));
    }

    #[test]
    fn external_backend_is_never_replaced_for_identity() {
        assert!(!should_respawn_for_runtime_identity(false, false));
    }
}
