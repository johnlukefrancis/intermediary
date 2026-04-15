// Path: src-tauri/src/lib/agent/supervisor/lifecycle.rs
// Description: Host-agent-first supervisor lifecycle implementation with optional Windows WSL backend

use super::{
    build_result, wsl_mode::wsl_backend_mode_requires_managed_owner, AgentSupervisor,
    EnsureProcessResult,
};
use crate::agent::install::resolve_launch_bundle;
use crate::agent::supervisor::runtime::{
    resolve_expected_dirs, resolve_wsl_port, should_prefer_installed_bundle,
};
use crate::agent::supervisor::state::ProcessKind;
use crate::agent::types::{
    AgentSupervisorConfig, AgentSupervisorResult, AgentSupervisorStatus, AgentSupervisorWslStatus,
};
use crate::agent::AgentWebSocketAuthState;
use crate::obs::logging;
use tauri::{AppHandle, Manager};

impl AgentSupervisor {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn ensure_running(
        &self,
        app: &AppHandle,
        config: AgentSupervisorConfig,
    ) -> Result<AgentSupervisorResult, String> {
        self.ensure_running_internal(app, config, false).await
    }

    pub async fn restart(
        &self,
        app: &AppHandle,
        config: AgentSupervisorConfig,
    ) -> Result<AgentSupervisorResult, String> {
        self.stop().await?;
        self.ensure_running_internal(app, config, true).await
    }

    pub async fn stop(&self) -> Result<(), String> {
        self.stop_process(ProcessKind::Host).await?;
        self.stop_process(ProcessKind::Wsl).await?;
        logging::log("info", "agent", "stop", "Agent processes stopped");
        Ok(())
    }

    async fn ensure_running_internal(
        &self,
        app: &AppHandle,
        config: AgentSupervisorConfig,
        force: bool,
    ) -> Result<AgentSupervisorResult, String> {
        if config.port < 1024 {
            return Err("Agent port must be >= 1024".to_string());
        }
        let supports_wsl = cfg!(target_os = "windows");
        let requested_wsl = config.wsl.as_ref().is_some_and(|wsl| wsl.required);
        let requires_wsl = supports_wsl && requested_wsl;
        let wsl_port = resolve_wsl_port(config.port, requires_wsl)?;
        let websocket_auth = app.state::<AgentWebSocketAuthState>().snapshot();
        let wsl_status = wsl_supervisor_status(supports_wsl, requires_wsl, wsl_port);
        let unsupported_wsl_message = requested_wsl
            .then_some("WSL backend launch is only supported on Windows hosts".to_string())
            .filter(|_| !supports_wsl);
        let (agent_dir, log_dir) = resolve_expected_dirs(app)?;

        if !config.auto_start && !force {
            if !requires_wsl {
                self.stop_process(ProcessKind::Wsl).await?;
            }
            return Ok(build_result(
                AgentSupervisorStatus::Disabled,
                config.port,
                supports_wsl,
                wsl_status.clone(),
                agent_dir,
                log_dir,
                unsupported_wsl_message.clone(),
            ));
        }

        let host_listening = self.probe_listening(config.port).await?;
        let wsl_listening = if requires_wsl {
            self.probe_listening(wsl_port).await?
        } else {
            false
        };
        let host_auth_ok = if host_listening {
            self.probe_websocket_auth(config.port, &websocket_auth.host_ws_token)
                .await?
        } else {
            false
        };
        let host_origin_compat_ok = if host_auth_ok {
            self.probe_websocket_origin_compatibility(
                config.port,
                &websocket_auth.host_ws_token,
                &websocket_auth.host_allowed_origins,
            )
            .await?
        } else {
            false
        };
        let wsl_auth_ok = if requires_wsl && wsl_listening {
            self.probe_websocket_auth(wsl_port, &websocket_auth.wsl_ws_token)
                .await?
        } else {
            false
        };

        if host_auth_ok && host_origin_compat_ok && !requires_wsl {
            self.stop_process(ProcessKind::Wsl).await?;
            self.set_last_error(None)?;
            return Ok(build_result(
                AgentSupervisorStatus::AlreadyRunning,
                config.port,
                supports_wsl,
                wsl_status.clone(),
                agent_dir,
                log_dir,
                unsupported_wsl_message.clone(),
            ));
        }
        if should_short_circuit_already_running_with_wsl(
            host_auth_ok,
            host_origin_compat_ok,
            requires_wsl,
            wsl_auth_ok,
            wsl_backend_mode_requires_managed_owner(),
        ) {
            self.set_last_error(None)?;
            return Ok(build_result(
                AgentSupervisorStatus::AlreadyRunning,
                config.port,
                supports_wsl,
                wsl_status.clone(),
                agent_dir,
                log_dir,
                unsupported_wsl_message.clone(),
            ));
        }

        let resource_dir = app
            .path()
            .resource_dir()
            .map_err(|_| "Failed to resolve resource directory".to_string())?;
        let app_local_data = app
            .path()
            .app_local_data_dir()
            .map_err(|_| "Failed to resolve app local data directory".to_string())?;
        let prefer_installed_bundle = should_prefer_installed_bundle(host_listening, wsl_listening);

        let bundle = tauri::async_runtime::spawn_blocking(move || {
            resolve_launch_bundle(&resource_dir, &app_local_data, prefer_installed_bundle)
        })
        .await
        .map_err(|err| format!("Agent bundle install task failed: {err}"))??;

        let host_result = self
            .ensure_host_running(&bundle, config.port, wsl_port, &websocket_auth, force)
            .await?;
        if host_result == EnsureProcessResult::Backoff {
            return Ok(build_result(
                AgentSupervisorStatus::Backoff,
                config.port,
                supports_wsl,
                wsl_status.clone(),
                agent_dir,
                log_dir,
                merge_supervisor_message(
                    "Host agent launch backoff active".to_string(),
                    unsupported_wsl_message.clone(),
                ),
            ));
        }

        let mut wsl_result = EnsureProcessResult::AlreadyRunning;
        if requires_wsl {
            let distro = config.wsl.as_ref().and_then(|wsl| wsl.distro.as_deref());
            wsl_result = self
                .ensure_wsl_running(&bundle, distro, wsl_port, &websocket_auth, force)
                .await?;
            if wsl_result == EnsureProcessResult::Backoff {
                return Ok(build_result(
                    AgentSupervisorStatus::Backoff,
                    config.port,
                    supports_wsl,
                    wsl_status.clone(),
                    agent_dir,
                    log_dir,
                    Some("WSL backend launch backoff active".to_string()),
                ));
            }
        } else {
            self.stop_process(ProcessKind::Wsl).await?;
        }

        let status = if host_result == EnsureProcessResult::Started
            || wsl_result == EnsureProcessResult::Started
        {
            AgentSupervisorStatus::Started
        } else {
            AgentSupervisorStatus::AlreadyRunning
        };

        self.set_last_error(None)?;
        Ok(build_result(
            status,
            config.port,
            supports_wsl,
            wsl_status,
            agent_dir,
            log_dir,
            unsupported_wsl_message,
        ))
    }
}

fn wsl_supervisor_status(
    supports_wsl: bool,
    requires_wsl: bool,
    wsl_port: u16,
) -> Option<AgentSupervisorWslStatus> {
    if !supports_wsl {
        return None;
    }
    Some(AgentSupervisorWslStatus {
        required: requires_wsl,
        port: wsl_port,
    })
}

fn merge_supervisor_message(primary: String, secondary: Option<String>) -> Option<String> {
    match secondary {
        Some(secondary) => Some(format!("{primary}. {secondary}")),
        None => Some(primary),
    }
}

fn should_short_circuit_already_running_with_wsl(
    host_auth_ok: bool,
    host_origin_compat_ok: bool,
    requires_wsl: bool,
    wsl_auth_ok: bool,
    managed_owner_required: bool,
) -> bool {
    host_auth_ok && host_origin_compat_ok && requires_wsl && wsl_auth_ok && !managed_owner_required
}

#[cfg(test)]
mod tests {
    use super::should_short_circuit_already_running_with_wsl;

    #[test]
    fn managed_mode_disables_wsl_already_running_fast_path() {
        assert!(!should_short_circuit_already_running_with_wsl(
            true, true, true, true, true
        ));
    }

    #[test]
    fn non_managed_mode_keeps_wsl_already_running_fast_path() {
        assert!(should_short_circuit_already_running_with_wsl(
            true, true, true, true, false
        ));
    }

    #[test]
    fn origin_mismatch_disables_wsl_already_running_fast_path() {
        assert!(!should_short_circuit_already_running_with_wsl(
            true, false, true, true, false
        ));
    }
}
