// Path: src-tauri/src/lib/commands/agent_control.rs
// Description: Tauri commands to manage the WSL agent supervisor

use crate::agent::{AgentSupervisor, AgentSupervisorConfig, AgentSupervisorResult};
use crate::obs::logging;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn ensure_agent_running(
    app: AppHandle,
    supervisor: State<'_, AgentSupervisor>,
    config: AgentSupervisorConfig,
) -> Result<AgentSupervisorResult, String> {
    supervisor
        .ensure_running(&app, config)
        .await
        .map_err(|err| {
            logging::log("error", "agent", "ensure_failed", &err);
            err
        })
}

#[tauri::command]
pub async fn restart_agent(
    app: AppHandle,
    supervisor: State<'_, AgentSupervisor>,
    config: AgentSupervisorConfig,
) -> Result<AgentSupervisorResult, String> {
    supervisor.restart(&app, config).await.map_err(|err| {
        logging::log("error", "agent", "restart_failed", &err);
        err
    })
}

#[tauri::command]
pub async fn stop_agent(supervisor: State<'_, AgentSupervisor>) -> Result<(), String> {
    supervisor.stop().await.map_err(|err| {
        logging::log("error", "agent", "stop_failed", &err);
        err
    })
}
