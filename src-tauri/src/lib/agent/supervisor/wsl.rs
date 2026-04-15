// Path: src-tauri/src/lib/agent/supervisor/wsl.rs
// Description: WSL backend startup and ownership detection for the supervisor

use super::wsl_mode::{
    backend_mode_allows_owner, resolve_wsl_backend_mode, WslBackendMode, WslBackendOwner,
};
use super::{AgentSupervisor, EnsureProcessResult};
use crate::agent::install::AgentBundlePaths;
use crate::agent::process_control::{capture_log_cursor, wait_for_agent_ready};
use crate::agent::supervisor::state::ProcessKind;
use crate::agent::types::AgentWebSocketAuth;
use crate::agent::wsl_process_control::{
    build_wsl_launch_target, format_wsl_target, spawn_wsl_agent_process,
};
use crate::obs::logging;
use std::process::Child;

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
            let owner = self.detect_wsl_backend_owner(&target).await?;
            if owner == WslBackendOwner::ExternalUnmanaged {
                self.reconcile_recorded_child(ProcessKind::Wsl, "external_unmanaged_detected")
                    .await?;
            }
            log_wsl_owner_detection(backend_mode, owner, wsl_port, &target);

            if !backend_mode_allows_owner(backend_mode, owner) {
                let message = AgentSupervisor::managed_mode_error_for_external_occupant(
                    backend_mode,
                    wsl_port,
                    &target,
                )
                .expect("managed-mode owner mismatch must produce an error");
                log_wsl_owner_mismatch(backend_mode, wsl_port, &target);
                return Err(message);
            }

            if self
                .probe_websocket_auth(wsl_port, &auth.wsl_ws_token)
                .await?
            {
                if owner == WslBackendOwner::InstalledManaged
                    && !matches!(backend_mode, WslBackendMode::External)
                {
                    self.set_wsl_launch_target(Some(target.clone()))?;
                }
                return Ok(EnsureProcessResult::AlreadyRunning);
            }

            if let Some(error) = self.wsl_auth_failure_error(backend_mode, owner, wsl_port, &target)
            {
                return Err(error);
            }

            self.set_wsl_launch_target(Some(target.clone()))?;
            self.remediate_stale_wsl_port(wsl_port, &target).await?;
        } else if matches!(backend_mode, WslBackendMode::External) {
            return Err(format!(
                "WSL backend mode=external requires an externally managed backend listening on port {wsl_port} ({})",
                format_wsl_target(&target)
            ));
        }

        self.set_wsl_launch_target(Some(target.clone()))?;
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
    ) -> Result<WslBackendOwner, String> {
        let installed_pid_count = self.detect_installed_wsl_pid_count(target).await?;
        Ok(if installed_pid_count > 0 {
            WslBackendOwner::InstalledManaged
        } else {
            WslBackendOwner::ExternalUnmanaged
        })
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
            (_, WslBackendOwner::InstalledManaged) => None,
        }
    }
}

async fn spawn_wsl_supervised(
    supervisor: &AgentSupervisor,
    bundle: &AgentBundlePaths,
    target: &crate::agent::wsl_process_control::WslLaunchTarget,
    wsl_port: u16,
    wsl_ws_token: &str,
) -> Result<EnsureProcessResult, String> {
    let target_summary = format_wsl_target(target);
    logging::log(
        "info",
        "agent",
        "spawn_start",
        &format!("kind=wsl port={wsl_port} {target_summary}"),
    );
    let bundle_for_spawn = bundle.clone();
    let target_for_spawn = target.clone();
    let wsl_ws_token = wsl_ws_token.to_string();
    let spawned = tauri::async_runtime::spawn_blocking(move || -> Result<Child, String> {
        let log_file = bundle_for_spawn.log_dir_host.join("agent_latest.log");
        let log_offset = capture_log_cursor(&log_file);
        let mut child = spawn_wsl_agent_process(
            &bundle_for_spawn,
            &target_for_spawn,
            wsl_port,
            &wsl_ws_token,
        )?;
        wait_for_agent_ready(
            &mut child,
            wsl_port,
            ProcessKind::Wsl.label(),
            &log_file,
            log_offset,
        )?;
        Ok(child)
    })
    .await
    .map_err(|err| format!("WSL agent spawn task failed: {err}"))?;

    match spawned {
        Ok(child) => {
            let pid = child.id();
            supervisor.replace_child(ProcessKind::Wsl, child).await?;
            supervisor.update_last_spawn(ProcessKind::Wsl)?;
            logging::log(
                "info",
                "agent",
                "spawn_ready",
                &format!("kind=wsl port={wsl_port} pid={pid} {target_summary}"),
            );
            Ok(EnsureProcessResult::Started)
        }
        Err(err) => {
            logging::log(
                "error",
                "agent",
                "spawn_exit_early",
                &format!("kind=wsl port={wsl_port} {target_summary} error={err}"),
            );
            supervisor.set_last_error(Some(err.clone()))?;
            Err(err)
        }
    }
}

fn log_wsl_owner_detection(
    backend_mode: WslBackendMode,
    owner: WslBackendOwner,
    wsl_port: u16,
    target: &crate::agent::wsl_process_control::WslLaunchTarget,
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

fn log_wsl_owner_mismatch(
    backend_mode: WslBackendMode,
    wsl_port: u16,
    target: &crate::agent::wsl_process_control::WslLaunchTarget,
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

fn log_wsl_external_auth_failure(
    mode: &str,
    owner: WslBackendOwner,
    wsl_port: u16,
    target: &crate::agent::wsl_process_control::WslLaunchTarget,
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
