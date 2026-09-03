// Path: src-tauri/src/lib/agent/supervisor/wsl_spawn.rs
// Description: Starting the WSL backend and recording it, with the stdin pipe that outlives the launch

use super::{AgentSupervisor, EnsureProcessResult};
use crate::agent::install::AgentBundlePaths;
use crate::agent::process_control::{capture_log_cursor, wait_for_agent_ready};
use crate::agent::supervisor::state::ProcessKind;
use crate::agent::wsl_process_control::{
    format_wsl_target, spawn_wsl_agent_process, WslLaunchTarget,
};
use crate::obs::logging;
use std::process::Child;

pub(super) async fn spawn_wsl_supervised(
    supervisor: &AgentSupervisor,
    bundle: &AgentBundlePaths,
    target: &WslLaunchTarget,
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
                &format!("kind=wsl port={wsl_port} pid={pid} stdin_pipe=held {target_summary}"),
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
