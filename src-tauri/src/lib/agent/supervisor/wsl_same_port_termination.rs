// Path: src-tauri/src/lib/agent/supervisor/wsl_same_port_termination.rs
// Description: Same-port Intermediary WSL agent termination for supervisor remediation

use super::wsl_runtime::{WSL_TERMINATE_POLL, WSL_TERMINATE_TERM_GRACE};
use super::AgentSupervisor;
use crate::agent::wsl_process_control::{
    format_wsl_target, terminate_intermediary_wsl_agent_processes_by_port, WslLaunchTarget,
    WslTerminateOutcome,
};
use crate::obs::logging;

impl AgentSupervisor {
    pub(super) async fn terminate_wsl_backend_same_port_intermediary(
        &self,
        target: &WslLaunchTarget,
        wsl_port: u16,
        reason: &str,
        attempt: usize,
    ) -> Result<(), String> {
        let target_summary = format_wsl_target(target);
        logging::log(
            "info",
            "agent",
            "wsl_terminate_start",
            &format!(
                "reason={reason} attempt={attempt} owner=same_port_intermediary port={wsl_port} {target_summary}"
            ),
        );
        let target_for_kill = target.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            terminate_intermediary_wsl_agent_processes_by_port(
                &target_for_kill,
                wsl_port,
                WSL_TERMINATE_TERM_GRACE,
                WSL_TERMINATE_POLL,
            )
        })
        .await
        .map_err(|err| format!("WSL same-port termination task failed: {err}"))?;

        match result {
            Ok(WslTerminateOutcome::NoMatch) => log_same_port_done(
                "info",
                reason,
                attempt,
                wsl_port,
                "no_match",
                &target_summary,
            ),
            Ok(WslTerminateOutcome::TerminatedWithTerm) => {
                log_same_port_done("info", reason, attempt, wsl_port, "term", &target_summary)
            }
            Ok(WslTerminateOutcome::TerminatedWithKill) => {
                log_same_port_done("warn", reason, attempt, wsl_port, "kill", &target_summary)
            }
            Err(err) => {
                logging::log(
                    "error",
                    "agent",
                    "wsl_terminate_done",
                    &format!(
                        "reason={reason} attempt={attempt} owner=same_port_intermediary outcome=failed port={wsl_port} {target_summary} error={err}"
                    ),
                );
                return Err(err);
            }
        }

        Ok(())
    }
}

fn log_same_port_done(
    level: &str,
    reason: &str,
    attempt: usize,
    wsl_port: u16,
    outcome: &str,
    target_summary: &str,
) {
    logging::log(
        level,
        "agent",
        "wsl_terminate_done",
        &format!(
            "reason={reason} attempt={attempt} owner=same_port_intermediary outcome={outcome} port={wsl_port} {target_summary}"
        ),
    );
}
