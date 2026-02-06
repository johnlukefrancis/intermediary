// Path: src-tauri/src/lib/agent/supervisor/lifecycle.rs
// Description: Dual-agent supervisor lifecycle implementation

use super::{build_result, AgentSupervisor, EnsureProcessResult};
use crate::agent::install::resolve_launch_bundle;
use crate::agent::supervisor_helpers::{
    resolve_expected_dirs, resolve_wsl_port, should_prefer_installed_bundle, ProcessKind,
};
use crate::agent::types::{AgentSupervisorConfig, AgentSupervisorResult, AgentSupervisorStatus};
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
        let wsl_port = resolve_wsl_port(config.port, config.requires_wsl)?;
        let (agent_dir, log_dir) = resolve_expected_dirs(app)?;

        if !config.auto_start && !force {
            if !config.requires_wsl {
                self.stop_process(ProcessKind::Wsl).await?;
            }
            return Ok(build_result(
                AgentSupervisorStatus::Disabled,
                config.port,
                wsl_port,
                config.requires_wsl,
                agent_dir,
                log_dir,
                None,
            ));
        }

        let host_listening = self.probe_listening(config.port).await?;
        let wsl_listening = if config.requires_wsl {
            self.probe_listening(wsl_port).await?
        } else {
            false
        };

        if host_listening && !config.requires_wsl {
            self.stop_process(ProcessKind::Wsl).await?;
            self.set_last_error(None)?;
            return Ok(build_result(
                AgentSupervisorStatus::AlreadyRunning,
                config.port,
                wsl_port,
                config.requires_wsl,
                agent_dir,
                log_dir,
                None,
            ));
        }
        if host_listening && wsl_listening {
            self.set_last_error(None)?;
            return Ok(build_result(
                AgentSupervisorStatus::AlreadyRunning,
                config.port,
                wsl_port,
                config.requires_wsl,
                agent_dir,
                log_dir,
                None,
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
            .ensure_host_running(&bundle, config.port, wsl_port, force)
            .await?;
        if host_result == EnsureProcessResult::Backoff {
            return Ok(build_result(
                AgentSupervisorStatus::Backoff,
                config.port,
                wsl_port,
                config.requires_wsl,
                agent_dir,
                log_dir,
                Some("Host agent launch backoff active".to_string()),
            ));
        }

        let mut wsl_result = EnsureProcessResult::AlreadyRunning;
        if config.requires_wsl {
            wsl_result = self
                .ensure_wsl_running(&bundle, config.distro.as_deref(), wsl_port, force)
                .await?;
            if wsl_result == EnsureProcessResult::Backoff {
                return Ok(build_result(
                    AgentSupervisorStatus::Backoff,
                    config.port,
                    wsl_port,
                    config.requires_wsl,
                    agent_dir,
                    log_dir,
                    Some("WSL agent launch backoff active".to_string()),
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
            wsl_port,
            config.requires_wsl,
            agent_dir,
            log_dir,
            None,
        ))
    }
}
