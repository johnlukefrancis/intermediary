// Path: src-tauri/src/lib/agent/wsl_agent_termination_channel.rs
// Description: The live in-distro channel an emergency WSL stop signals through

//! The production half of [`super::wsl_agent_termination`]: every call here is
//! one `bash --noprofile --norc -s` script fed to `wsl.exe` over stdin, so the
//! script bytes never cross the Windows→WSL argument marshalling. The
//! orchestration above it holds no `wsl.exe` knowledge at all, which is what
//! lets its drain envelope and escalation be tested without a distro.

use super::wsl_agent_termination::WslTerminationChannel;
use super::wsl_process_control_commands::build_wsl_signal_pids_command_line;
use super::wsl_process_tree_commands::{
    build_wsl_kill_agent_process_trees_command_line, count_signalled_process_trees,
};
use crate::wsl_control::{distro_label, run_wsl_script, sanitize_stream_text};

pub(super) struct LiveChannel<'a, F> {
    distro: Option<&'a str>,
    list: F,
}

impl<'a, F> LiveChannel<'a, F>
where
    F: FnMut() -> Result<Vec<u32>, String>,
{
    /// `list` is the detector this particular stop is responsible for — exact
    /// binary path, port env signature, or port listener — so one channel serves
    /// every caller without a second termination route per detector.
    pub(super) fn new(distro: Option<&'a str>, list: F) -> Self {
        Self { distro, list }
    }
}

impl<F> WslTerminationChannel for LiveChannel<'_, F>
where
    F: FnMut() -> Result<Vec<u32>, String>,
{
    fn list_pids(&mut self) -> Result<Vec<u32>, String> {
        (self.list)()
    }

    fn send_term(&mut self, pids: &[u32]) -> Result<Option<String>, String> {
        let command = build_wsl_signal_pids_command_line(pids, "TERM");
        run_signal_command(self.distro, &command, "TERM")
    }

    fn kill_process_trees(&mut self, pids: &[u32]) -> Result<usize, String> {
        let command = build_wsl_kill_agent_process_trees_command_line(pids);
        let output = run_wsl_script(self.distro, &command)?;
        if !output.status.success() {
            let code = output
                .status
                .code()
                .map(|value| value.to_string())
                .unwrap_or_else(|| "signal".to_string());
            return Err(format!(
                "WSL agent process-tree KILL failed (exit={code}, distro={}): {}",
                distro_label(self.distro),
                sanitize_stream_text(&String::from_utf8_lossy(&output.stderr))
            ));
        }
        Ok(count_signalled_process_trees(&String::from_utf8_lossy(
            &output.stdout,
        )))
    }
}

fn run_signal_command(
    distro: Option<&str>,
    command: &str,
    stage: &str,
) -> Result<Option<String>, String> {
    let output = run_wsl_script(distro, command)?;
    if output.status.success() {
        return Ok(None);
    }
    let status = output
        .status
        .code()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "signal".to_string());
    let stderr = sanitize_stream_text(&String::from_utf8_lossy(&output.stderr));
    let prefix = format!(
        "WSL agent {stage} command failed (exit={status}, distro={})",
        distro_label(distro)
    );
    Ok(Some(if stderr.is_empty() {
        prefix
    } else {
        format!("{prefix}: {stderr}")
    }))
}
