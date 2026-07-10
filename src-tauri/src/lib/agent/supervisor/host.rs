// Path: src-tauri/src/lib/agent/supervisor/host.rs
// Description: Host-agent startup and stale-port remediation for the supervisor

use super::{AgentSupervisor, EnsureProcessResult};
use crate::agent::host_process_control::{terminate_host_agent_process, HostTerminateOutcome};
use crate::agent::install::AgentBundlePaths;
use crate::agent::process_control::{
    capture_log_cursor, spawn_host_agent_process, wait_for_agent_ready,
};
use crate::agent::supervisor::process_kill::{KILL_WAIT_POLL, KILL_WAIT_TIMEOUT};
use crate::agent::supervisor::state::ProcessKind;
use crate::agent::AgentWebSocketAuth;
use crate::obs::logging;
use std::path::Path;
use std::process::Child;

impl AgentSupervisor {
    pub(super) async fn ensure_host_running(
        &self,
        bundle: &AgentBundlePaths,
        host_port: u16,
        wsl_port: u16,
        auth: &AgentWebSocketAuth,
        force: bool,
    ) -> Result<EnsureProcessResult, String> {
        if self.probe_listening(host_port).await? {
            let identity = self
                .probe_websocket_identity(host_port, &auth.host_ws_token)
                .await?;
            let auth_ok = identity.authenticated;
            let runtime_matches = identity.matches_runtime(&bundle.host_agent_sha256);
            let origin_compat_ok = if auth_ok {
                self.probe_websocket_origin_compatibility(
                    host_port,
                    &auth.host_ws_token,
                    &auth.host_allowed_origins,
                )
                .await?
            } else {
                false
            };

            if should_adopt_running_host(force, auth_ok, origin_compat_ok, runtime_matches) {
                return Ok(EnsureProcessResult::AlreadyRunning);
            }

            let remediation_reason = if auth_ok && origin_compat_ok && !runtime_matches {
                "runtime_identity_mismatch"
            } else if auth_ok && origin_compat_ok {
                "runtime_replacement"
            } else if auth_ok {
                "origin_probe_failed"
            } else {
                "auth_probe_failed"
            };
            self.reconcile_recorded_child(ProcessKind::Host, remediation_reason)
                .await?;
            if self.probe_listening(host_port).await? {
                self.remediate_stale_host_port(
                    host_port,
                    &bundle.host_agent_binary_host,
                    remediation_reason,
                )
                .await?;
            }
            if self.probe_listening(host_port).await? {
                let message = if auth_ok && origin_compat_ok && !runtime_matches {
                    format!(
                        "Host agent port {host_port} is occupied by a process whose executable identity does not match the packaged runtime and could not be retired"
                    )
                } else if auth_ok && origin_compat_ok {
                    format!(
                        "Host agent port {host_port} is occupied by a process that could not be retired for runtime replacement"
                    )
                } else if auth_ok {
                    format!(
                        "Host agent port {host_port} is occupied by a process that rejected the current websocket origin contract"
                    )
                } else {
                    format!(
                        "Host agent port {host_port} is occupied by a process that rejected the current websocket token"
                    )
                };
                return Err(message);
            }
        }
        self.reconcile_recorded_child(ProcessKind::Host, "port_probe_failed")
            .await?;
        if !force && self.is_in_backoff(ProcessKind::Host)? {
            return Ok(EnsureProcessResult::Backoff);
        }

        logging::log(
            "info",
            "agent",
            "spawn_start",
            &format!("kind=host port={host_port}"),
        );
        let bundle_for_spawn = bundle.clone();
        let auth = auth.clone();
        let spawned = tauri::async_runtime::spawn_blocking(move || -> Result<Child, String> {
            let log_file = bundle_for_spawn.log_dir_host.join("agent_latest.log");
            let log_offset = capture_log_cursor(&log_file);
            let mut child = spawn_host_agent_process(
                &bundle_for_spawn,
                host_port,
                wsl_port,
                &auth.host_ws_token,
                &auth.wsl_ws_token,
                &auth.host_allowed_origins,
            )?;
            wait_for_agent_ready(
                &mut child,
                host_port,
                ProcessKind::Host.label(),
                &log_file,
                log_offset,
            )?;
            Ok(child)
        })
        .await
        .map_err(|err| format!("Host agent spawn task failed: {err}"))?;

        match spawned {
            Ok(child) => {
                let pid = child.id();
                self.replace_child(ProcessKind::Host, child).await?;
                self.update_last_spawn(ProcessKind::Host)?;
                logging::log(
                    "info",
                    "agent",
                    "spawn_ready",
                    &format!("kind=host port={host_port} pid={pid}"),
                );
                Ok(EnsureProcessResult::Started)
            }
            Err(err) => {
                logging::log(
                    "error",
                    "agent",
                    "spawn_exit_early",
                    &format!("kind=host port={host_port} error={err}"),
                );
                self.set_last_error(Some(err.clone()))?;
                Err(err)
            }
        }
    }

    async fn remediate_stale_host_port(
        &self,
        host_port: u16,
        binary_path: &Path,
        reason: &str,
    ) -> Result<(), String> {
        let binary_path = binary_path.to_path_buf();
        logging::log(
            "info",
            "agent",
            "host_terminate_start",
            &format!(
                "reason={reason} port={host_port} binary={}",
                binary_path.display()
            ),
        );
        let result = tauri::async_runtime::spawn_blocking(move || {
            terminate_host_agent_process(&binary_path, KILL_WAIT_TIMEOUT, KILL_WAIT_POLL)
        })
        .await
        .map_err(|err| format!("Host stale-port remediation task failed: {err}"))?;

        match result {
            Ok(HostTerminateOutcome::NoMatch) => {
                logging::log(
                    "info",
                    "agent",
                    "host_terminate_done",
                    &format!("reason={reason} port={host_port} outcome=no_match"),
                );
                Ok(())
            }
            Ok(HostTerminateOutcome::Terminated) => {
                logging::log(
                    "warn",
                    "agent",
                    "host_terminate_done",
                    &format!("reason={reason} port={host_port} outcome=terminated"),
                );
                Ok(())
            }
            Err(err) => {
                logging::log(
                    "error",
                    "agent",
                    "host_terminate_done",
                    &format!("reason={reason} port={host_port} outcome=failed error={err}"),
                );
                Err(err)
            }
        }
    }
}

fn should_adopt_running_host(
    force: bool,
    auth_ok: bool,
    origin_compat_ok: bool,
    runtime_matches: bool,
) -> bool {
    !force && auth_ok && origin_compat_ok && runtime_matches
}

#[cfg(test)]
mod tests {
    use super::should_adopt_running_host;

    #[test]
    fn healthy_current_host_is_adopted() {
        assert!(should_adopt_running_host(false, true, true, true));
    }

    #[test]
    fn healthy_stale_host_is_retired_for_replacement() {
        assert!(!should_adopt_running_host(true, true, true, true));
    }

    #[test]
    fn healthy_process_with_stale_executable_is_retired() {
        assert!(!should_adopt_running_host(false, true, true, false));
    }
}
