// Path: src-tauri/src/lib/agent/process_control.rs
// Description: Spawn helpers for host/WSL agents and readiness probing

use super::install::AgentBundlePaths;
use crate::commands::agent_probe::probe_port_blocking;
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const READY_TIMEOUT: Duration = Duration::from_secs(30);
const READY_POLL: Duration = Duration::from_millis(250);

pub fn spawn_host_agent_process(
    bundle: &AgentBundlePaths,
    host_port: u16,
    wsl_port: u16,
) -> Result<Child, String> {
    if !bundle.host_agent_binary_windows.is_file() {
        return Err(format!(
            "Host agent binary is missing: {}",
            bundle.host_agent_binary_windows.display()
        ));
    }

    let mut command = Command::new(&bundle.host_agent_binary_windows);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
        .current_dir(&bundle.agent_dir_windows)
        .env("INTERMEDIARY_AGENT_PORT", host_port.to_string())
        .env("INTERMEDIARY_WSL_AGENT_PORT", wsl_port.to_string())
        .env("INTERMEDIARY_AGENT_VERSION", &bundle.version)
        .env(
            "INTERMEDIARY_AGENT_LOG_DIR",
            path_to_string(&bundle.log_dir_windows)?,
        )
        .spawn()
        .map_err(|err| format!("Failed to spawn host agent: {err}"))
}

pub fn spawn_wsl_agent_process(
    bundle: &AgentBundlePaths,
    distro: Option<&str>,
    wsl_port: u16,
) -> Result<Child, String> {
    let mut command = Command::new("wsl.exe");
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    if let Some(name) = distro {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            command.args(["-d", trimmed]);
        }
    }

    let env_port = wsl_port.to_string();
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

pub fn wait_for_agent_ready(child: &mut Child, port: u16, label: &str) -> Result<(), String> {
    let start = Instant::now();
    let mut last_error: Option<String> = None;

    while start.elapsed() < READY_TIMEOUT {
        if let Some(status) = child
            .try_wait()
            .map_err(|err| format!("Failed to poll {label} process: {err}"))?
        {
            return Err(format!("{label} exited early: {status}"));
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
        "{label} did not become ready on port {port} within {}ms{detail}",
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

fn path_to_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .ok_or_else(|| "Path contains invalid UTF-8".to_string())
        .map(|value| value.to_string())
}
