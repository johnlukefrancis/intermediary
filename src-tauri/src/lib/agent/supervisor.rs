// Path: src-tauri/src/lib/agent/supervisor.rs
// Description: App-managed WSL agent supervisor with spawn and stop support

use super::install::{ensure_agent_bundle, AgentBundlePaths};
use super::types::{AgentSupervisorConfig, AgentSupervisorResult, AgentSupervisorStatus};
use crate::commands::agent_probe::probe_port_blocking;
use crate::obs::logging;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

const SPAWN_BACKOFF: Duration = Duration::from_millis(1500);
const READY_TIMEOUT: Duration = Duration::from_secs(30);
const READY_POLL: Duration = Duration::from_millis(250);

#[derive(Debug, Default)]
struct AgentSupervisorState {
    child: Option<Child>,
    last_spawn_at: Option<Instant>,
    last_error: Option<String>,
}

#[derive(Debug, Default)]
pub struct AgentSupervisor {
    state: Mutex<AgentSupervisorState>,
}

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
        let mut child = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
            state.child.take()
        };

        if let Some(mut child) = child.take() {
            tauri::async_runtime::spawn_blocking(move || {
                let _ = child.kill();
                let _ = child.wait();
            })
            .await
            .map_err(|err| format!("Agent stop task failed: {err}"))?;
        }

        logging::log("info", "agent", "stop", "Agent process stopped");
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

        let (agent_dir, log_dir) = resolve_expected_dirs(app)?;

        if !config.auto_start && !force {
            return Ok(AgentSupervisorResult {
                status: AgentSupervisorStatus::Disabled,
                port: config.port,
                agent_dir,
                log_dir,
                message: None,
            });
        }

        let probe = tauri::async_runtime::spawn_blocking(move || probe_port_blocking(config.port))
            .await
            .map_err(|err| format!("Agent probe task failed: {err}"))?;

        if probe.listening {
            self.set_last_error(None)?;
            return Ok(AgentSupervisorResult {
                status: AgentSupervisorStatus::AlreadyRunning,
                port: config.port,
                agent_dir,
                log_dir,
                message: None,
            });
        }

        if !force && self.is_in_backoff()? {
            return Ok(AgentSupervisorResult {
                status: AgentSupervisorStatus::Backoff,
                port: config.port,
                agent_dir,
                log_dir,
                message: Some("Agent launch backoff active".to_string()),
            });
        }

        let resource_dir = app
            .path()
            .resource_dir()
            .map_err(|_| "Failed to resolve resource directory".to_string())?;
        let app_local_data = app
            .path()
            .app_local_data_dir()
            .map_err(|_| "Failed to resolve app local data directory".to_string())?;

        let bundle = tauri::async_runtime::spawn_blocking(move || {
            ensure_agent_bundle(&resource_dir, &app_local_data)
        })
        .await
        .map_err(|err| format!("Agent bundle install task failed: {err}"))??;

        let bundle_for_spawn = bundle.clone();
        let config_for_spawn = config.clone();
        let child = tauri::async_runtime::spawn_blocking(move || -> Result<Child, String> {
            let mut child = spawn_agent_process(&bundle_for_spawn, &config_for_spawn)?;
            wait_for_agent_ready(&mut child, config_for_spawn.port)?;
            Ok(child)
        })
        .await
        .map_err(|err| format!("Agent spawn task failed: {err}"))?;

        let child = match child {
            Ok(child) => child,
            Err(err) => {
                self.set_last_error(Some(err.clone()))?;
                return Err(err);
            }
        };

        self.store_child(child)?;
        self.update_last_spawn()?;
        self.set_last_error(None)?;

        logging::log(
            "info",
            "agent",
            "start",
            &format!("Spawned WSL agent on port {}", config.port),
        );

        Ok(AgentSupervisorResult {
            status: AgentSupervisorStatus::Started,
            port: config.port,
            agent_dir: bundle.agent_dir_windows.display().to_string(),
            log_dir: bundle.log_dir_windows.display().to_string(),
            message: None,
        })
    }

    fn is_in_backoff(&self) -> Result<bool, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        match state.last_spawn_at {
            Some(last) => Ok(last.elapsed() < SPAWN_BACKOFF),
            None => Ok(false),
        }
    }

    fn update_last_spawn(&self) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        state.last_spawn_at = Some(Instant::now());
        Ok(())
    }

    fn store_child(&self, child: Child) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        state.child = Some(child);
        Ok(())
    }

    fn set_last_error(&self, message: Option<String>) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        state.last_error = message;
        Ok(())
    }
}

fn resolve_expected_dirs(app: &AppHandle) -> Result<(String, String), String> {
    let app_local_data = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "Failed to resolve app local data directory".to_string())?;
    let agent_dir = app_local_data.join("agent");
    let log_dir = app_local_data.join("logs");
    Ok((agent_dir.display().to_string(), log_dir.display().to_string()))
}

fn spawn_agent_process(
    bundle: &AgentBundlePaths,
    config: &AgentSupervisorConfig,
) -> Result<Child, String> {
    let mut command = Command::new("wsl.exe");
    if let Some(name) = &config.distro {
        if !name.trim().is_empty() {
            command.args(["-d", name.trim()]);
        }
    }

    let env_port = config.port.to_string();
    let env_version = quote_bash(&bundle.version);
    let env_log = quote_bash(&bundle.log_dir_wsl);
    let agent_dir = quote_bash(&bundle.agent_dir_wsl);
    let command_line = format!(
        "cd {agent_dir} && chmod +x ./im_agent && INTERMEDIARY_AGENT_PORT={env_port} INTERMEDIARY_AGENT_VERSION={env_version} INTERMEDIARY_AGENT_LOG_DIR={env_log} ./im_agent"
    );

    command
        .args(["--", "bash", "-lc", &command_line])
        .spawn()
        .map_err(|err| format!("Failed to spawn WSL agent: {err}"))
}

fn wait_for_agent_ready(child: &mut Child, port: u16) -> Result<(), String> {
    let start = Instant::now();
    let mut last_error: Option<String> = None;

    while start.elapsed() < READY_TIMEOUT {
        if let Some(status) = child
            .try_wait()
            .map_err(|err| format!("Failed to poll agent process: {err}"))?
        {
            return Err(format!("WSL agent exited early: {status}"));
        }

        let probe = probe_port_blocking(port);
        if probe.listening {
            return Ok(());
        }

        last_error = probe.error;
        std::thread::sleep(READY_POLL);
    }

    let _ = child.kill();
    let _ = child.wait();

    let detail = last_error
        .map(|err| format!(" ({err})"))
        .unwrap_or_default();
    Err(format!(
        "WSL agent did not become ready on port {port} within {}ms{detail}",
        READY_TIMEOUT.as_millis()
    ))
}

fn quote_bash(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    let escaped = value.replace('\'', "'\"'\"'");
    format!("'{escaped}'")
}
