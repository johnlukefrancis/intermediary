// Path: src-tauri/src/lib/agent/supervisor/shutdown.rs
// Description: App-exit teardown: stop agents, then free WSL VM RAM when the distro is idle

use super::AgentSupervisor;
use crate::agent::wsl_shutdown::{probe_wsl_distro_idle, terminate_wsl_distro};
use crate::obs::logging;

impl AgentSupervisor {
    /// Called from the Tauri exit handler. Reliably stops the managed agents, then — only when the
    /// backend is one we manage and no interactive WSL session is open — terminates the distro so
    /// the WSL VM releases its RAM instead of lingering. Any teardown failure is logged, never
    /// propagated, so exit is never blocked.
    pub async fn shutdown_on_exit(&self) -> Result<(), String> {
        // Snapshot before stop(): `stop` clears the launch target, and we need the distro/port to
        // decide the conditional teardown. `last_wsl_backend` is None in external mode, so a
        // user-managed backend is never torn down here.
        let backend = self.last_wsl_backend_snapshot()?;
        let stop_result = self.stop().await;

        if let Some(handle) = backend {
            if let Err(err) = self
                .free_wsl_ram_if_idle(handle.distro.as_deref(), handle.port)
                .await
            {
                logging::log(
                    "warn",
                    "agent",
                    "wsl_exit_teardown",
                    &format!("outcome=failed error={err}"),
                );
            }
        }

        stop_result
    }

    async fn free_wsl_ram_if_idle(
        &self,
        distro: Option<&str>,
        wsl_port: u16,
    ) -> Result<(), String> {
        // stop() already reclaimed our agent by port, so nothing of ours needs excluding.
        let distro_for_probe = distro.map(str::to_string);
        let status = tauri::async_runtime::spawn_blocking(move || {
            probe_wsl_distro_idle(distro_for_probe.as_deref(), &[])
        })
        .await
        .map_err(|err| format!("WSL idle probe task failed: {err}"))??;

        if !status.idle {
            logging::log(
                "info",
                "agent",
                "wsl_exit_teardown",
                &format!("outcome=skipped reason=interactive_session_open port={wsl_port}"),
            );
            return Ok(());
        }

        let Some(distro_name) = distro.map(str::to_string).or(status.distro_name) else {
            logging::log(
                "info",
                "agent",
                "wsl_exit_teardown",
                "outcome=skipped reason=unknown_distro",
            );
            return Ok(());
        };

        let distro_for_kill = distro_name.clone();
        tauri::async_runtime::spawn_blocking(move || terminate_wsl_distro(&distro_for_kill))
            .await
            .map_err(|err| format!("WSL distro terminate task failed: {err}"))??;

        logging::log(
            "info",
            "agent",
            "wsl_exit_teardown",
            &format!("outcome=terminated distro={distro_name} reason=idle"),
        );
        Ok(())
    }
}
