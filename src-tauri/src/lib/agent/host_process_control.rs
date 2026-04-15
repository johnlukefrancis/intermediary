// Path: src-tauri/src/lib/agent/host_process_control.rs
// Description: Windows host-agent process detection and stale-port termination helpers

use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const HOST_COMMAND_TIMEOUT: Duration = Duration::from_secs(5);
const HOST_COMMAND_POLL: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostTerminateOutcome {
    NoMatch,
    Terminated,
}

pub fn terminate_host_agent_process(
    binary_path: &Path,
    term_grace: Duration,
    poll: Duration,
) -> Result<HostTerminateOutcome, String> {
    if !cfg!(target_os = "windows") {
        return Ok(HostTerminateOutcome::NoMatch);
    }

    let mut matching_pids = list_exact_host_agent_pids(binary_path)?;
    if matching_pids.is_empty() {
        return Ok(HostTerminateOutcome::NoMatch);
    }

    run_powershell_script(&build_stop_process_command(&matching_pids))?;
    if wait_for_host_agent_exit(binary_path, term_grace, poll)? {
        return Ok(HostTerminateOutcome::Terminated);
    }

    matching_pids = list_exact_host_agent_pids(binary_path)?;
    if matching_pids.is_empty() {
        return Ok(HostTerminateOutcome::Terminated);
    }

    Err(format!(
        "Host agent process matched by {} did not exit after Stop-Process",
        binary_path.display()
    ))
}

fn wait_for_host_agent_exit(
    binary_path: &Path,
    term_grace: Duration,
    poll: Duration,
) -> Result<bool, String> {
    let start = Instant::now();
    loop {
        if !probe_host_agent_running(binary_path)? {
            return Ok(true);
        }
        if start.elapsed() >= term_grace {
            return Ok(false);
        }
        thread::sleep(poll);
    }
}

fn probe_host_agent_running(binary_path: &Path) -> Result<bool, String> {
    Ok(!list_exact_host_agent_pids(binary_path)?.is_empty())
}

pub fn list_exact_host_agent_pids(binary_path: &Path) -> Result<Vec<u32>, String> {
    if !cfg!(target_os = "windows") {
        return Ok(Vec::new());
    }

    let binary_path = binary_path.to_str().ok_or_else(|| {
        format!(
            "Host agent path contains invalid UTF-8: {}",
            binary_path.display()
        )
    })?;

    let output = run_powershell_script(&build_list_pids_command(binary_path))?;
    parse_pid_list(&String::from_utf8_lossy(&output.stdout))
}

fn build_list_pids_command(binary_path: &str) -> String {
    let escaped_path = escape_powershell_single_quoted(binary_path);
    format!(
        concat!(
            "$target = [System.IO.Path]::GetFullPath('{}'); ",
            "Get-CimInstance Win32_Process | ",
            "Where-Object {{ $_.ExecutablePath -and ",
            "[System.StringComparer]::OrdinalIgnoreCase.Equals(",
            "[System.IO.Path]::GetFullPath($_.ExecutablePath), $target) }} | ",
            "ForEach-Object {{ $_.ProcessId }}"
        ),
        escaped_path
    )
}

fn build_stop_process_command(pids: &[u32]) -> String {
    let joined = pids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<String>>()
        .join(",");
    format!("Stop-Process -Id {joined} -Force -ErrorAction Stop")
}

fn run_powershell_script(script: &str) -> Result<Output, String> {
    let mut command = Command::new("powershell.exe");
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|err| format!("Failed to execute PowerShell host-agent command: {err}"))?;
    let start = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child
                    .wait_with_output()
                    .map_err(|err| format!("Failed to read PowerShell output: {err}"));
            }
            Ok(None) => {
                if start.elapsed() >= HOST_COMMAND_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "Host-agent PowerShell command timed out after {}ms",
                        HOST_COMMAND_TIMEOUT.as_millis()
                    ));
                }
                thread::sleep(HOST_COMMAND_POLL);
            }
            Err(err) => {
                return Err(format!(
                    "Failed to poll PowerShell host-agent command: {err}"
                ));
            }
        }
    }
}

fn parse_pid_list(raw: &str) -> Result<Vec<u32>, String> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.parse::<u32>()
                .map_err(|err| format!("Failed to parse host agent pid '{line}': {err}"))
        })
        .collect()
}

fn escape_powershell_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}
