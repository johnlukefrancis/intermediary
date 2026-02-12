// Path: src-tauri/src/lib/agent/supervisor/wsl.rs
// Description: WSL backend lifecycle helpers, including stale-port remediation and in-WSL termination

use super::{AgentSupervisor, EnsureProcessResult};
use crate::agent::install::AgentBundlePaths;
use crate::agent::process_control::{capture_log_cursor, wait_for_agent_ready};
use crate::agent::supervisor_helpers::{
    ProcessKind, WSL_STALE_RETRY_BACKOFF, WSL_TERMINATE_POLL, WSL_TERMINATE_TERM_GRACE,
};
use crate::agent::types::AgentWebSocketAuth;
use crate::agent::wsl_process_control::{
    build_wsl_launch_target, format_wsl_target, list_exact_wsl_agent_pids, spawn_wsl_agent_process,
    terminate_wsl_agent_process, WslLaunchTarget, WslTerminateOutcome,
};
use crate::obs::logging;
use std::process::Child;

const WSL_BACKEND_MODE_ENV: &str = "INTERMEDIARY_WSL_BACKEND_MODE";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WslBackendMode {
    Auto,
    Managed,
    External,
}

impl WslBackendMode {
    fn log_key(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Managed => "managed",
            Self::External => "external",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WslBackendOwner {
    InstalledManaged,
    ExternalUnmanaged,
}

impl WslBackendOwner {
    fn log_key(self) -> &'static str {
        match self {
            Self::InstalledManaged => "installed_managed",
            Self::ExternalUnmanaged => "external_unmanaged",
        }
    }
}

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
                    "invalid={invalid} env={WSL_BACKEND_MODE_ENV} defaulted=auto {}",
                    format_wsl_target(&target)
                ),
            );
        }

        if self.probe_listening(wsl_port).await? {
            let installed_pid_count = self.detect_installed_wsl_pid_count(&target).await?;
            let owner = if installed_pid_count > 0 {
                WslBackendOwner::InstalledManaged
            } else {
                WslBackendOwner::ExternalUnmanaged
            };
            if owner == WslBackendOwner::ExternalUnmanaged {
                self.reconcile_recorded_child(ProcessKind::Wsl, "external_unmanaged_detected")
                    .await?;
            }
            logging::log(
                "info",
                "agent",
                "wsl_owner_detected",
                &format!(
                    "mode={} owner={} installed_pid_count={installed_pid_count} port={wsl_port} {}",
                    backend_mode.log_key(),
                    owner.log_key(),
                    format_wsl_target(&target)
                ),
            );

            if !backend_mode_allows_owner(backend_mode, owner) {
                let message = format!(
                    "WSL backend mode=managed expected the installed backend process, but an external occupant is using port {wsl_port} ({})",
                    format_wsl_target(&target)
                );
                logging::log(
                    "warn",
                    "agent",
                    "wsl_external_unmanaged_auth_failed",
                    &format!(
                        "mode=managed owner=external_unmanaged port={wsl_port} {}",
                        format_wsl_target(&target)
                    ),
                );
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

            match (backend_mode, owner) {
                (WslBackendMode::External, _) => {
                    let message = format!(
                        "WSL backend auth failed on port {wsl_port} while mode=external (owner={}, {}). Ensure the external backend token matches app websocket auth state.",
                        owner.log_key(),
                        format_wsl_target(&target)
                    );
                    logging::log(
                        "warn",
                        "agent",
                        "wsl_external_unmanaged_auth_failed",
                        &format!(
                            "mode=external owner={} port={wsl_port} {}",
                            owner.log_key(),
                            format_wsl_target(&target)
                        ),
                    );
                    return Err(message);
                }
                (WslBackendMode::Auto, WslBackendOwner::ExternalUnmanaged) => {
                    let message = format!(
                        "WSL backend port {wsl_port} is occupied by an external process that rejected the current websocket token ({}) and will not be terminated in mode=auto.",
                        format_wsl_target(&target)
                    );
                    logging::log(
                        "warn",
                        "agent",
                        "wsl_external_unmanaged_auth_failed",
                        &format!(
                            "mode=auto owner=external_unmanaged port={wsl_port} {}",
                            format_wsl_target(&target)
                        ),
                    );
                    return Err(message);
                }
                (WslBackendMode::Managed, WslBackendOwner::ExternalUnmanaged) => {
                    let message = format!(
                        "WSL backend mode=managed expected the installed backend process, but an external occupant is using port {wsl_port} ({})",
                        format_wsl_target(&target)
                    );
                    logging::log(
                        "warn",
                        "agent",
                        "wsl_external_unmanaged_auth_failed",
                        &format!(
                            "mode=managed owner=external_unmanaged port={wsl_port} {}",
                            format_wsl_target(&target)
                        ),
                    );
                    return Err(message);
                }
                (_, WslBackendOwner::InstalledManaged) => {
                    self.set_wsl_launch_target(Some(target.clone()))?;
                    self.remediate_stale_wsl_port(wsl_port, &target).await?;
                }
            }
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

        let target_summary = format_wsl_target(&target);
        logging::log(
            "info",
            "agent",
            "spawn_start",
            &format!("kind=wsl port={wsl_port} {target_summary}"),
        );
        let bundle_for_spawn = bundle.clone();
        let target_for_spawn = target.clone();
        let wsl_ws_token = auth.wsl_ws_token.clone();
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
                self.replace_child(ProcessKind::Wsl, child).await?;
                self.update_last_spawn(ProcessKind::Wsl)?;
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
                self.set_last_error(Some(err.clone()))?;
                Err(err)
            }
        }
    }

    pub(super) async fn terminate_wsl_backend_for_reason(
        &self,
        reason: &str,
    ) -> Result<(), String> {
        let Some(target) = self.wsl_launch_target_snapshot()? else {
            return Ok(());
        };
        self.terminate_wsl_backend_target(&target, reason, 1).await
    }

    pub(super) fn set_wsl_launch_target(
        &self,
        target: Option<WslLaunchTarget>,
    ) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        state.wsl_launch_target = target;
        Ok(())
    }

    async fn remediate_stale_wsl_port(
        &self,
        wsl_port: u16,
        target: &WslLaunchTarget,
    ) -> Result<(), String> {
        for attempt in 1..=2 {
            let reason = if attempt == 1 {
                "auth_probe_failed"
            } else {
                "auth_probe_failed_retry"
            };

            self.reconcile_recorded_child(ProcessKind::Wsl, reason)
                .await?;
            self.terminate_wsl_backend_target(target, reason, attempt)
                .await?;
            if !self.probe_listening(wsl_port).await? {
                return Ok(());
            }

            if attempt == 1 {
                logging::log(
                    "info",
                    "agent",
                    "stale_retry_wait",
                    &format!(
                        "kind=wsl port={wsl_port} wait_ms={} {}",
                        WSL_STALE_RETRY_BACKOFF.as_millis(),
                        format_wsl_target(target)
                    ),
                );
                self.sleep_wsl_stale_retry_backoff().await?;
            }
        }

        Err(format!(
            "WSL agent port {wsl_port} is occupied by a process that rejected the current websocket token ({})",
            format_wsl_target(target)
        ))
    }

    async fn terminate_wsl_backend_target(
        &self,
        target: &WslLaunchTarget,
        reason: &str,
        attempt: usize,
    ) -> Result<(), String> {
        let target_summary = format_wsl_target(target);
        logging::log(
            "info",
            "agent",
            "wsl_terminate_start",
            &format!("reason={reason} attempt={attempt} {target_summary}"),
        );
        let target_for_kill = target.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            terminate_wsl_agent_process(
                &target_for_kill,
                WSL_TERMINATE_TERM_GRACE,
                WSL_TERMINATE_POLL,
            )
        })
        .await
        .map_err(|err| format!("WSL termination task failed: {err}"))?;

        match result {
            Ok(WslTerminateOutcome::NoMatch) => {
                logging::log(
                    "info",
                    "agent",
                    "wsl_terminate_done",
                    &format!("reason={reason} attempt={attempt} outcome=no_match {target_summary}"),
                );
                Ok(())
            }
            Ok(WslTerminateOutcome::TerminatedWithTerm) => {
                logging::log(
                    "info",
                    "agent",
                    "wsl_terminate_done",
                    &format!("reason={reason} attempt={attempt} outcome=term {target_summary}"),
                );
                Ok(())
            }
            Ok(WslTerminateOutcome::TerminatedWithKill) => {
                logging::log(
                    "warn",
                    "agent",
                    "wsl_terminate_done",
                    &format!("reason={reason} attempt={attempt} outcome=kill {target_summary}"),
                );
                Ok(())
            }
            Err(err) => {
                logging::log(
                    "error",
                    "agent",
                    "wsl_terminate_done",
                    &format!(
                        "reason={reason} attempt={attempt} outcome=failed {target_summary} error={err}"
                    ),
                );
                Err(err)
            }
        }
    }

    fn wsl_launch_target_snapshot(&self) -> Result<Option<WslLaunchTarget>, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        Ok(state.wsl_launch_target.clone())
    }

    async fn sleep_wsl_stale_retry_backoff(&self) -> Result<(), String> {
        tauri::async_runtime::spawn_blocking(move || std::thread::sleep(WSL_STALE_RETRY_BACKOFF))
            .await
            .map_err(|err| format!("WSL stale retry backoff task failed: {err}"))?;
        Ok(())
    }

    async fn detect_installed_wsl_pid_count(
        &self,
        target: &WslLaunchTarget,
    ) -> Result<usize, String> {
        let target_for_probe = target.clone();
        tauri::async_runtime::spawn_blocking(move || list_exact_wsl_agent_pids(&target_for_probe))
            .await
            .map_err(|err| format!("WSL owner-detection task failed: {err}"))?
            .map(|pids| pids.len())
    }
}

fn resolve_wsl_backend_mode() -> (WslBackendMode, Option<String>) {
    let raw = std::env::var(WSL_BACKEND_MODE_ENV).ok();
    parse_wsl_backend_mode(raw.as_deref())
}

pub(super) fn wsl_backend_mode_requires_managed_owner() -> bool {
    matches!(resolve_wsl_backend_mode().0, WslBackendMode::Managed)
}

fn parse_wsl_backend_mode(raw: Option<&str>) -> (WslBackendMode, Option<String>) {
    let Some(raw_mode) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return (WslBackendMode::Auto, None);
    };

    if raw_mode.eq_ignore_ascii_case("auto") {
        return (WslBackendMode::Auto, None);
    }
    if raw_mode.eq_ignore_ascii_case("managed") {
        return (WslBackendMode::Managed, None);
    }
    if raw_mode.eq_ignore_ascii_case("external") {
        return (WslBackendMode::External, None);
    }

    (WslBackendMode::Auto, Some(raw_mode.to_string()))
}

fn backend_mode_allows_owner(mode: WslBackendMode, owner: WslBackendOwner) -> bool {
    !matches!(
        (mode, owner),
        (WslBackendMode::Managed, WslBackendOwner::ExternalUnmanaged)
    )
}

#[cfg(test)]
mod tests {
    use super::{
        backend_mode_allows_owner, parse_wsl_backend_mode, WslBackendMode, WslBackendOwner,
    };

    #[test]
    fn parse_wsl_backend_mode_defaults_to_auto_when_missing_or_empty() {
        let (mode_missing, invalid_missing) = parse_wsl_backend_mode(None);
        assert_eq!(mode_missing, WslBackendMode::Auto);
        assert_eq!(invalid_missing, None);

        let (mode_empty, invalid_empty) = parse_wsl_backend_mode(Some("  "));
        assert_eq!(mode_empty, WslBackendMode::Auto);
        assert_eq!(invalid_empty, None);
    }

    #[test]
    fn parse_wsl_backend_mode_accepts_supported_values_case_insensitively() {
        let (auto_mode, auto_invalid) = parse_wsl_backend_mode(Some("AUTO"));
        assert_eq!(auto_mode, WslBackendMode::Auto);
        assert_eq!(auto_invalid, None);

        let (managed_mode, managed_invalid) = parse_wsl_backend_mode(Some("managed"));
        assert_eq!(managed_mode, WslBackendMode::Managed);
        assert_eq!(managed_invalid, None);

        let (external_mode, external_invalid) = parse_wsl_backend_mode(Some("External"));
        assert_eq!(external_mode, WslBackendMode::External);
        assert_eq!(external_invalid, None);
    }

    #[test]
    fn parse_wsl_backend_mode_reports_invalid_value() {
        let (mode, invalid) = parse_wsl_backend_mode(Some("sidecar"));
        assert_eq!(mode, WslBackendMode::Auto);
        assert_eq!(invalid, Some("sidecar".to_string()));
    }

    #[test]
    fn managed_mode_requires_installed_owner() {
        assert!(backend_mode_allows_owner(
            WslBackendMode::Auto,
            WslBackendOwner::ExternalUnmanaged
        ));
        assert!(backend_mode_allows_owner(
            WslBackendMode::External,
            WslBackendOwner::ExternalUnmanaged
        ));
        assert!(backend_mode_allows_owner(
            WslBackendMode::Managed,
            WslBackendOwner::InstalledManaged
        ));
        assert!(!backend_mode_allows_owner(
            WslBackendMode::Managed,
            WslBackendOwner::ExternalUnmanaged
        ));
    }
}
