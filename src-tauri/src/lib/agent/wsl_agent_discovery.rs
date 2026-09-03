// Path: src-tauri/src/lib/agent/wsl_agent_discovery.rs
// Description: In-distro discovery of the Intermediary WSL agent pids a stop is responsible for

use super::wsl_command_runner::{run_wsl_bash, sanitize_stream_text};
use super::wsl_process_control::{format_wsl_target, WslLaunchTarget};
use super::wsl_process_control_commands::distro_label;
use super::wsl_process_probe_commands::{
    build_wsl_list_exact_pids_command_line, build_wsl_list_intermediary_agent_pids_command_line,
    build_wsl_list_port_listener_pids_command_line,
};

pub fn list_exact_wsl_agent_pids(target: &WslLaunchTarget) -> Result<Vec<u32>, String> {
    let command_line = build_wsl_list_exact_pids_command_line(&target.agent_bin_wsl);
    let output = run_wsl_bash(target.distro.as_deref(), &command_line)?;
    if !output.status.success() {
        return Err(format!(
            "Failed to list matching WSL agent pids ({}, {}): {}",
            exit_label(&output.status),
            format_wsl_target(target),
            sanitize_stream_text(&String::from_utf8_lossy(&output.stderr))
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
        return Err(format!(
            "Failed to list same-port Intermediary WSL agent pids ({}, port={wsl_port}, {}): {}",
            exit_label(&output.status),
            format_wsl_target(target),
            sanitize_stream_text(&String::from_utf8_lossy(&output.stderr))
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
        return Err(format!(
            "Failed to list WSL agent port listeners ({}, port={wsl_port}, distro={}): {}",
            exit_label(&output.status),
            distro_label(distro),
            sanitize_stream_text(&String::from_utf8_lossy(&output.stderr))
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

fn exit_label(status: &std::process::ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("exit={code}"),
        None => "exit=signal".to_string(),
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
#[path = "wsl_agent_discovery_tests.rs"]
mod tests;
