// Path: src-tauri/src/lib/agent/process_control.rs
// Description: Spawn helpers for host/WSL agents and readiness probing

mod log_tail;

use super::install::AgentBundlePaths;
use crate::commands::agent_probe::probe_port_blocking;
use im_bundle::process_job::JobHandle;
use log_tail::format_early_exit_log;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::io::ErrorKind;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const READY_TIMEOUT: Duration = Duration::from_secs(30);
const READY_POLL: Duration = Duration::from_millis(250);

/// The host agent we started, together with the owner of every process it goes
/// on to start. They are returned as one value because they have one lifetime:
/// the supervisor records both or ends both.
pub struct SpawnedHostAgent {
    pub child: Child,
    pub job: JobHandle,
}

/// Starts the host agent inside a process-tree owner created *before* the spawn
/// and joined to the child immediately after it, so a later stop reaches
/// everything the agent started and not just the agent.
///
/// There is no unowned path: an agent we cannot own is an agent we cannot
/// reliably stop, and it is not started. Off Windows the owner is inert (see
/// `im_bundle::process_job`), so this is one code path on every platform.
pub fn spawn_host_agent_process(
    bundle: &AgentBundlePaths,
    host_port: u16,
    wsl_port: u16,
    host_ws_token: &str,
    wsl_ws_token: &str,
    host_ws_allowed_origins: &[String],
) -> Result<SpawnedHostAgent, String> {
    if !bundle.host_agent_binary_host.is_file() {
        return Err(format!(
            "Host agent binary is missing: {}",
            bundle.host_agent_binary_host.display()
        ));
    }
    let job = JobHandle::create().map_err(|err| {
        format!(
            "Failed to create the host agent's process tree owner (binary: {}): {err}",
            bundle.host_agent_binary_host.display()
        )
    })?;

    let mut command = Command::new(&bundle.host_agent_binary_host);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let host_allowed_origins = host_ws_allowed_origins.join(",");

    command
        .current_dir(&bundle.agent_dir_host)
        .env("INTERMEDIARY_AGENT_PORT", host_port.to_string())
        .env("INTERMEDIARY_WSL_AGENT_PORT", wsl_port.to_string())
        .env("INTERMEDIARY_HOST_WS_TOKEN", host_ws_token)
        .env("INTERMEDIARY_WSL_WS_TOKEN", wsl_ws_token)
        .env("INTERMEDIARY_HOST_WS_ALLOWED_ORIGINS", host_allowed_origins)
        .env("INTERMEDIARY_AGENT_VERSION", &bundle.version)
        .env(
            "INTERMEDIARY_AGENT_LOG_DIR",
            path_to_string(&bundle.log_dir_host)?,
        )
        .env("INTERMEDIARY_AGENT_STDIO_LOGGING", "0")
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut child = command
        .spawn()
        .map_err(|err| format_host_spawn_error(&bundle.host_agent_binary_host, err))?;
    if let Err(err) = job.assign(&child) {
        // The agent is running but unowned, which is exactly what must not be
        // recorded: end it here and report the failure on the spawn route.
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!(
            "Failed to join the host agent to its process tree owner (binary: {}): {err}",
            bundle.host_agent_binary_host.display()
        ));
    }
    Ok(SpawnedHostAgent { child, job })
}

pub fn wait_for_agent_ready(
    child: &mut Child,
    port: u16,
    label: &str,
    log_file: &Path,
    log_offset: u64,
) -> Result<(), String> {
    let start = Instant::now();
    let mut last_error: Option<String> = None;

    while start.elapsed() < READY_TIMEOUT {
        if let Some(status) = child
            .try_wait()
            .map_err(|err| format!("Failed to poll {label} process: {err}"))?
        {
            let detail = format_early_exit_log(log_file, log_offset);
            return Err(format_early_exit_error(label, status.to_string(), detail));
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

fn path_to_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .ok_or_else(|| "Path contains invalid UTF-8".to_string())
        .map(|value| value.to_string())
}

fn format_host_spawn_error(binary_path: &Path, err: std::io::Error) -> String {
    #[cfg(unix)]
    if err.kind() == ErrorKind::PermissionDenied {
        return format!(
            "Failed to spawn host agent: permission denied (binary: {}). Likely causes: executable bit missing, macOS quarantine attribute, or signing/notarization misconfiguration for bundled helper binaries. Original error: {err}",
            binary_path.display()
        );
    }

    format!(
        "Failed to spawn host agent (binary: {}): {err}",
        binary_path.display()
    )
}

fn format_early_exit_error(label: &str, status: String, detail: Option<String>) -> String {
    match detail {
        Some(detail) => format!("{label} exited early: {status}. {detail}"),
        None => format!("{label} exited early: {status}"),
    }
}

pub fn capture_log_cursor(log_file: &Path) -> u64 {
    match std::fs::metadata(log_file) {
        Ok(metadata) => metadata.len(),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => 0,
        Err(_) => 0,
    }
}
