// Path: src-tauri/src/lib/agent/wsl_shutdown.rs
// Description: Conditional WSL distro teardown to free VM RAM when no interactive session remains

use super::wsl_command_runner::{run_wsl_bash, sanitize_stream_text};
use super::wsl_process_control_commands::distro_label;
use super::wsl_process_probe_commands::build_wsl_idle_teardown_probe_command_line;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const WSL_TERMINATE_DISTRO_TIMEOUT: Duration = Duration::from_secs(10);
const WSL_TERMINATE_DISTRO_POLL: Duration = Duration::from_millis(50);

/// Whether the distro is otherwise idle (no interactive `pts/*` session), and the distro name as
/// reported by `WSL_DISTRO_NAME` inside it (used to target `wsl --terminate` when the caller did
/// not pin a distro).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WslDistroIdleStatus {
    pub idle: bool,
    pub distro_name: Option<String>,
}

/// Probes whether any interactive `pts/*` session is open in the distro, excluding `agent_pids`.
pub fn probe_wsl_distro_idle(
    distro: Option<&str>,
    agent_pids: &[u32],
) -> Result<WslDistroIdleStatus, String> {
    if !cfg!(target_os = "windows") {
        return Ok(WslDistroIdleStatus {
            idle: false,
            distro_name: None,
        });
    }

    let command_line = build_wsl_idle_teardown_probe_command_line(agent_pids);
    let output = run_wsl_bash(distro, &command_line)?;
    if !output.status.success() {
        let stderr = sanitize_stream_text(&String::from_utf8_lossy(&output.stderr));
        return Err(format!(
            "Failed to probe WSL distro idle state (distro={}): {stderr}",
            distro_label(distro)
        ));
    }

    Ok(parse_idle_probe_output(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

/// Terminates a specific WSL distro (`wsl --terminate <name>`) to release its share of the WSL VM
/// RAM. Targeted to the named distro; never `wsl --shutdown` (which would stop every distro).
pub fn terminate_wsl_distro(distro_name: &str) -> Result<(), String> {
    if !cfg!(target_os = "windows") {
        return Ok(());
    }

    let mut command = Command::new("wsl.exe");
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.arg("--terminate").arg(distro_name);

    let output = run_bounded_command(
        command,
        WSL_TERMINATE_DISTRO_TIMEOUT,
        WSL_TERMINATE_DISTRO_POLL,
        "WSL distro terminate",
    )?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = sanitize_stream_text(&String::from_utf8_lossy(&output.stderr));
    Err(format!("wsl --terminate {distro_name} failed: {stderr}"))
}

fn parse_idle_probe_output(raw: &str) -> WslDistroIdleStatus {
    // Expected: "idle <distro>" or "busy <distro>" on the last non-empty line.
    let line = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .next_back()
        .unwrap_or("");
    let mut parts = line.splitn(2, ' ');
    let verdict = parts.next().unwrap_or("");
    let distro_name = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    WslDistroIdleStatus {
        idle: verdict == "idle",
        distro_name,
    }
}

fn run_bounded_command(
    mut command: Command,
    timeout: Duration,
    poll: Duration,
    what: &str,
) -> Result<Output, String> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|err| format!("Failed to run {what}: {err}"))?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child
                    .wait_with_output()
                    .map_err(|err| format!("Failed to read {what} output: {err}"));
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("{what} timed out after {}ms", timeout.as_millis()));
                }
                thread::sleep(poll);
            }
            Err(err) => {
                return Err(format!("Failed to poll {what}: {err}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_idle_probe_output;

    #[test]
    fn parses_idle_verdict_with_distro_name() {
        let status = parse_idle_probe_output("idle Ubuntu-22.04\n");
        assert!(status.idle);
        assert_eq!(status.distro_name.as_deref(), Some("Ubuntu-22.04"));
    }

    #[test]
    fn parses_busy_verdict() {
        let status = parse_idle_probe_output("busy Ubuntu\n");
        assert!(!status.idle);
        assert_eq!(status.distro_name.as_deref(), Some("Ubuntu"));
    }

    #[test]
    fn idle_without_distro_name_is_still_idle() {
        let status = parse_idle_probe_output("idle \n");
        assert!(status.idle);
        assert_eq!(status.distro_name, None);
    }

    #[test]
    fn ignores_leading_noise_lines() {
        let status = parse_idle_probe_output("some warning\nidle Ubuntu\n");
        assert!(status.idle);
        assert_eq!(status.distro_name.as_deref(), Some("Ubuntu"));
    }
}
