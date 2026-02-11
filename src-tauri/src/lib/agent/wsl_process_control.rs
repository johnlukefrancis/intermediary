// Path: src-tauri/src/lib/agent/wsl_process_control.rs
// Description: WSL agent launch target resolution, spawning, and in-WSL termination helpers

use super::install::AgentBundlePaths;
use super::wsl_process_control_commands::{
    build_wsl_bash_args, build_wsl_probe_command_line, build_wsl_signal_command_line,
    build_wsl_spawn_command_line, distro_label, normalize_distro,
};
use crate::paths::wsl_convert::windows_to_wsl_path;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const WSL_AGENT_BINARY_NAME: &str = "im_agent";
const WSL_COMMAND_TIMEOUT: Duration = Duration::from_secs(5);
const WSL_COMMAND_POLL: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WslLaunchTarget {
    pub distro: Option<String>,
    pub agent_dir_wsl: String,
    pub agent_bin_wsl: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WslTerminateOutcome {
    NoMatch,
    TerminatedWithTerm,
    TerminatedWithKill,
}

pub fn build_wsl_launch_target(
    bundle: &AgentBundlePaths,
    distro: Option<&str>,
) -> Result<WslLaunchTarget, String> {
    if !cfg!(target_os = "windows") {
        return Err("WSL agent launch is only supported on Windows hosts".to_string());
    }

    let wsl_agent_binary = bundle
        .wsl_agent_binary_host
        .as_ref()
        .ok_or_else(|| "WSL agent binary is not available for this platform".to_string())?;
    if !wsl_agent_binary.is_file() {
        return Err(format!(
            "WSL agent binary is missing: {}",
            wsl_agent_binary.display()
        ));
    }

    let agent_dir_host = path_to_string(&bundle.agent_dir_host)?;
    let agent_dir_wsl = windows_to_wsl_path(&agent_dir_host).ok_or_else(|| {
        format!("Failed to convert host agent directory to WSL path: {agent_dir_host}")
    })?;
    let agent_bin_wsl = format!("{agent_dir_wsl}/{WSL_AGENT_BINARY_NAME}");

    Ok(WslLaunchTarget {
        distro: normalize_distro(distro),
        agent_dir_wsl,
        agent_bin_wsl,
    })
}

pub fn spawn_wsl_agent_process(
    bundle: &AgentBundlePaths,
    target: &WslLaunchTarget,
    wsl_port: u16,
    wsl_ws_token: &str,
) -> Result<Child, String> {
    let log_dir_host = path_to_string(&bundle.log_dir_host)?;
    let log_dir_wsl = windows_to_wsl_path(&log_dir_host).ok_or_else(|| {
        format!("Failed to convert host log directory to WSL path: {log_dir_host}")
    })?;

    let command_line = build_wsl_spawn_command_line(
        &target.agent_bin_wsl,
        wsl_port,
        wsl_ws_token,
        &bundle.version,
        &log_dir_wsl,
    );
    let mut command = build_wsl_bash_command(target.distro.as_deref(), &command_line);

    command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| format_wsl_spawn_error(err, target.distro.as_deref()))
}

pub fn terminate_wsl_agent_process(
    target: &WslLaunchTarget,
    term_grace: Duration,
    poll: Duration,
) -> Result<WslTerminateOutcome, String> {
    if !cfg!(target_os = "windows") {
        return Ok(WslTerminateOutcome::NoMatch);
    }

    if !probe_wsl_agent_running(target)? {
        return Ok(WslTerminateOutcome::NoMatch);
    }

    let term_command = build_wsl_signal_command_line(&target.agent_bin_wsl, "TERM");
    run_wsl_bash_checked(target.distro.as_deref(), &term_command, "TERM")?;
    if wait_for_wsl_agent_exit(target, term_grace, poll)? {
        return Ok(WslTerminateOutcome::TerminatedWithTerm);
    }

    let kill_command = build_wsl_signal_command_line(&target.agent_bin_wsl, "KILL");
    run_wsl_bash_checked(target.distro.as_deref(), &kill_command, "KILL")?;
    if wait_for_wsl_agent_exit(target, term_grace, poll)? {
        return Ok(WslTerminateOutcome::TerminatedWithKill);
    }

    Err(format!(
        "WSL agent process matched by {} did not exit after TERM/KILL",
        target.agent_bin_wsl
    ))
}

pub fn format_wsl_target(target: &WslLaunchTarget) -> String {
    let distro = target.distro.as_deref().unwrap_or("default");
    format!("distro={distro} agent_bin_wsl={}", target.agent_bin_wsl)
}

fn wait_for_wsl_agent_exit(
    target: &WslLaunchTarget,
    term_grace: Duration,
    poll: Duration,
) -> Result<bool, String> {
    let start = Instant::now();
    loop {
        if !probe_wsl_agent_running(target)? {
            return Ok(true);
        }
        if start.elapsed() >= term_grace {
            return Ok(false);
        }
        thread::sleep(poll);
    }
}

fn probe_wsl_agent_running(target: &WslLaunchTarget) -> Result<bool, String> {
    let command_line = build_wsl_probe_command_line(&target.agent_bin_wsl);
    let output = run_wsl_bash(target.distro.as_deref(), &command_line)?;
    if output.status.success() {
        return Ok(true);
    }
    if output.status.code() == Some(1) {
        return Ok(false);
    }

    let stderr = sanitize_stream_text(&String::from_utf8_lossy(&output.stderr));
    let status = output
        .status
        .code()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "signal".to_string());
    Err(format!(
        "Failed to query WSL agent process state (exit={status}, {}): {stderr}",
        format_wsl_target(target)
    ))
}

fn run_wsl_bash_checked(
    distro: Option<&str>,
    command_line: &str,
    stage: &str,
) -> Result<(), String> {
    let output = run_wsl_bash(distro, command_line)?;
    if output.status.success() {
        return Ok(());
    }

    let status = output
        .status
        .code()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "signal".to_string());
    let stderr = sanitize_stream_text(&String::from_utf8_lossy(&output.stderr));
    Err(format!(
        "WSL agent {stage} command failed (exit={status}, distro={}): {stderr}",
        distro_label(distro)
    ))
}

fn run_wsl_bash(distro: Option<&str>, command_line: &str) -> Result<Output, String> {
    let mut command = build_wsl_bash_command(distro, command_line);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|err| {
        format!(
            "Failed to execute WSL command (distro={}): {err}",
            distro_label(distro)
        )
    })?;
    let start = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child.wait_with_output().map_err(|err| {
                    format!(
                        "Failed to read WSL command output (distro={}): {err}",
                        distro_label(distro)
                    )
                });
            }
            Ok(None) => {
                if start.elapsed() >= WSL_COMMAND_TIMEOUT {
                    return timeout_wsl_command(child, distro);
                }
                thread::sleep(WSL_COMMAND_POLL);
            }
            Err(err) => {
                return Err(format!(
                    "Failed to poll WSL command process (distro={}): {err}",
                    distro_label(distro)
                ));
            }
        }
    }
}

fn timeout_wsl_command(mut child: Child, distro: Option<&str>) -> Result<Output, String> {
    let _ = child.kill();
    let _ = child.wait();
    Err(format!(
        "WSL command timed out after {}ms (distro={})",
        WSL_COMMAND_TIMEOUT.as_millis(),
        distro_label(distro)
    ))
}

fn build_wsl_bash_command(distro: Option<&str>, command_line: &str) -> Command {
    let mut command = Command::new("wsl.exe");
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command.args(build_wsl_bash_args(distro, command_line));
    command
}

fn path_to_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .ok_or_else(|| "Path contains invalid UTF-8".to_string())
        .map(|value| value.to_string())
}

fn format_wsl_spawn_error(err: std::io::Error, distro: Option<&str>) -> String {
    let distro_name = distro_label(distro);
    match err.kind() {
        std::io::ErrorKind::NotFound => format!(
            "Failed to spawn WSL agent: wsl.exe was not found. Ensure WSL is installed and available on PATH (distro={distro_name}). Original error: {err}"
        ),
        std::io::ErrorKind::PermissionDenied => format!(
            "Failed to spawn WSL agent: permission denied while starting wsl.exe (distro={distro_name}). Original error: {err}"
        ),
        _ => format!("Failed to spawn WSL agent (distro={distro_name}): {err}"),
    }
}

fn sanitize_stream_text(text: &str) -> String {
    text.trim()
        .replace('\r', "")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}
