// Path: src-tauri/src/lib/agent/wsl_process_control.rs
// Description: WSL agent launch target resolution, spawning, and in-WSL termination helpers

use super::install::AgentBundlePaths;
use super::wsl_command_runner::{run_wsl_bash, run_wsl_signal_command, sanitize_stream_text};
use super::wsl_process_control_commands::{
    build_wsl_bash_args, build_wsl_list_exact_pids_command_line,
    build_wsl_list_intermediary_agent_pids_command_line,
    build_wsl_list_port_listener_pids_command_line, build_wsl_signal_pids_command_line,
    build_wsl_spawn_command_line, distro_label, normalize_distro,
};
use crate::paths::wsl_convert::windows_to_wsl_path;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const WSL_AGENT_BINARY_NAME: &str = "im_agent";

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

    terminate_matching_wsl_agent_processes(
        target.distro.as_deref(),
        term_grace,
        poll,
        || list_exact_wsl_agent_pids(target),
        &target.agent_bin_wsl,
    )
}

pub fn terminate_intermediary_wsl_agent_processes_by_port(
    target: &WslLaunchTarget,
    wsl_port: u16,
    term_grace: Duration,
    poll: Duration,
) -> Result<WslTerminateOutcome, String> {
    if !cfg!(target_os = "windows") {
        return Ok(WslTerminateOutcome::NoMatch);
    }

    let distro = target.distro.clone();
    terminate_matching_wsl_agent_processes(
        distro.as_deref(),
        term_grace,
        poll,
        || list_reclaimable_wsl_agent_pids_by_port(target, wsl_port),
        &format!("INTERMEDIARY_AGENT_PORT={wsl_port} or port-listener :{wsl_port}"),
    )
}

/// Reclaims (TERM→KILL) any Intermediary `im_agent` bound to `wsl_port`, using only the
/// port-listener probe. Used by config-less callers (`stop`, app exit) that know the distro and
/// port but may not hold a full launch target for the running backend.
pub fn terminate_wsl_agent_by_port_listener(
    distro: Option<&str>,
    wsl_port: u16,
    term_grace: Duration,
    poll: Duration,
) -> Result<WslTerminateOutcome, String> {
    if !cfg!(target_os = "windows") {
        return Ok(WslTerminateOutcome::NoMatch);
    }

    terminate_matching_wsl_agent_processes(
        distro,
        term_grace,
        poll,
        || list_wsl_agent_pids_by_port_listener(distro, wsl_port),
        &format!("port-listener :{wsl_port}"),
    )
}

fn terminate_matching_wsl_agent_processes<F>(
    distro: Option<&str>,
    term_grace: Duration,
    poll: Duration,
    mut list_pids: F,
    match_description: &str,
) -> Result<WslTerminateOutcome, String>
where
    F: FnMut() -> Result<Vec<u32>, String>,
{
    let mut matching_pids = list_pids()?;
    if matching_pids.is_empty() {
        return Ok(WslTerminateOutcome::NoMatch);
    }

    let mut signal_errors: Vec<String> = Vec::new();

    let term_command = build_wsl_signal_pids_command_line(&matching_pids, "TERM");
    if let Some(error) = run_wsl_signal_command(distro, &term_command, "TERM")? {
        signal_errors.push(error);
    }
    if wait_for_wsl_agent_exit(term_grace, poll, &mut list_pids)? {
        return Ok(WslTerminateOutcome::TerminatedWithTerm);
    }

    matching_pids = list_pids()?;
    if matching_pids.is_empty() {
        return Ok(WslTerminateOutcome::TerminatedWithTerm);
    }

    let kill_command = build_wsl_signal_pids_command_line(&matching_pids, "KILL");
    if let Some(error) = run_wsl_signal_command(distro, &kill_command, "KILL")? {
        signal_errors.push(error);
    }
    if wait_for_wsl_agent_exit(term_grace, poll, &mut list_pids)? {
        return Ok(WslTerminateOutcome::TerminatedWithKill);
    }

    let mut error =
        format!("WSL agent process matched by {match_description} did not exit after TERM/KILL");
    if !signal_errors.is_empty() {
        error = format!("{error}. {}", signal_errors.join("; "));
    }

    Err(error)
}

pub fn format_wsl_target(target: &WslLaunchTarget) -> String {
    let distro = target.distro.as_deref().unwrap_or("default");
    format!("distro={distro} agent_bin_wsl={}", target.agent_bin_wsl)
}

pub fn list_exact_wsl_agent_pids(target: &WslLaunchTarget) -> Result<Vec<u32>, String> {
    let command_line = build_wsl_list_exact_pids_command_line(&target.agent_bin_wsl);
    let output = run_wsl_bash(target.distro.as_deref(), &command_line)?;
    if !output.status.success() {
        let status = output
            .status
            .code()
            .map(|value| value.to_string())
            .unwrap_or_else(|| "signal".to_string());
        let stderr = sanitize_stream_text(&String::from_utf8_lossy(&output.stderr));
        return Err(format!(
            "Failed to list matching WSL agent pids (exit={status}, {}): {stderr}",
            format_wsl_target(target)
        ));
    }

    parse_pid_list(&String::from_utf8_lossy(&output.stdout))
}

pub fn list_intermediary_wsl_agent_pids_by_port(
    target: &WslLaunchTarget,
    wsl_port: u16,
) -> Result<Vec<u32>, String> {
    let command_line = build_wsl_list_intermediary_agent_pids_command_line(wsl_port);
    let output = run_wsl_bash(target.distro.as_deref(), &command_line)?;
    if !output.status.success() {
        let status = output
            .status
            .code()
            .map(|value| value.to_string())
            .unwrap_or_else(|| "signal".to_string());
        let stderr = sanitize_stream_text(&String::from_utf8_lossy(&output.stderr));
        return Err(format!(
            "Failed to list same-port Intermediary WSL agent pids (exit={status}, port={wsl_port}, {}): {stderr}",
            format_wsl_target(target)
        ));
    }

    parse_pid_list(&String::from_utf8_lossy(&output.stdout))
}

/// Lists PIDs that are (a) TCP listeners on `wsl_port` and (b) confirmed Intermediary `im_agent`
/// processes. Uses `ss` inside the distro; if `ss` is unavailable it yields an empty list rather
/// than erroring, so callers degrade to the path/env detectors.
pub fn list_wsl_agent_pids_by_port_listener(
    distro: Option<&str>,
    wsl_port: u16,
) -> Result<Vec<u32>, String> {
    let command_line = build_wsl_list_port_listener_pids_command_line(wsl_port);
    let output = run_wsl_bash(distro, &command_line)?;
    if !output.status.success() {
        let status = output
            .status
            .code()
            .map(|value| value.to_string())
            .unwrap_or_else(|| "signal".to_string());
        let stderr = sanitize_stream_text(&String::from_utf8_lossy(&output.stderr));
        return Err(format!(
            "Failed to list WSL agent port listeners (exit={status}, port={wsl_port}, distro={}): {stderr}",
            distro_label(distro)
        ));
    }

    parse_pid_list(&String::from_utf8_lossy(&output.stdout))
}

/// Union of the env-signature detector and the port-listener detector: every Intermediary
/// `im_agent` reachable on `wsl_port`, regardless of how it was launched. This is the authoritative
/// reclamation set for a stale/mismatched backend occupying our reserved port.
pub fn list_reclaimable_wsl_agent_pids_by_port(
    target: &WslLaunchTarget,
    wsl_port: u16,
) -> Result<Vec<u32>, String> {
    let mut pids = list_intermediary_wsl_agent_pids_by_port(target, wsl_port)?;
    pids.extend(list_wsl_agent_pids_by_port_listener(
        target.distro.as_deref(),
        wsl_port,
    )?);
    pids.sort_unstable();
    pids.dedup();
    Ok(pids)
}

fn wait_for_wsl_agent_exit<F>(
    term_grace: Duration,
    poll: Duration,
    list_pids: &mut F,
) -> Result<bool, String>
where
    F: FnMut() -> Result<Vec<u32>, String>,
{
    let start = Instant::now();
    loop {
        if list_pids()?.is_empty() {
            return Ok(true);
        }
        if start.elapsed() >= term_grace {
            return Ok(false);
        }
        thread::sleep(poll);
    }
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

fn parse_pid_list(raw: &str) -> Result<Vec<u32>, String> {
    let mut parsed: Vec<u32> = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let pid = trimmed
            .parse::<u32>()
            .map_err(|_| format!("Invalid pid entry returned by WSL command: {trimmed}"))?;
        parsed.push(pid);
    }

    parsed.sort_unstable();
    parsed.dedup();
    Ok(parsed)
}

#[cfg(test)]
#[path = "wsl_process_control_tests.rs"]
mod tests;
