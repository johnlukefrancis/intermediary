// Path: src-tauri/src/lib/agent/wsl_command_runner.rs
// Description: Bounded WSL command execution helpers for agent process control

use super::wsl_process_control_commands::{build_wsl_bash_args, distro_label};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const WSL_COMMAND_TIMEOUT: Duration = Duration::from_secs(5);
const WSL_COMMAND_POLL: Duration = Duration::from_millis(25);

pub(super) fn run_wsl_signal_command(
    distro: Option<&str>,
    command_line: &str,
    stage: &str,
) -> Result<Option<String>, String> {
    let output = run_wsl_bash(distro, command_line)?;
    if output.status.success() {
        return Ok(None);
    }

    let stderr = sanitize_stream_text(&String::from_utf8_lossy(&output.stderr));
    Ok(Some(format_wsl_signal_command_error(
        stage,
        distro,
        output.status.code(),
        &stderr,
    )))
}

pub(super) fn run_wsl_bash(distro: Option<&str>, command_line: &str) -> Result<Output, String> {
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

pub(super) fn sanitize_stream_text(text: &str) -> String {
    text.trim()
        .replace('\r', "")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
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

fn format_wsl_signal_command_error(
    stage: &str,
    distro: Option<&str>,
    status: Option<i32>,
    stderr: &str,
) -> String {
    let status = status
        .map(|value| value.to_string())
        .unwrap_or_else(|| "signal".to_string());
    let prefix = format!(
        "WSL agent {stage} command failed (exit={status}, distro={})",
        distro_label(distro)
    );
    if stderr.is_empty() {
        return prefix;
    }
    format!("{prefix}: {stderr}")
}

#[cfg(test)]
mod tests {
    use super::format_wsl_signal_command_error;

    #[test]
    fn format_wsl_signal_command_error_omits_empty_stderr_suffix() {
        let error = format_wsl_signal_command_error("TERM", Some("Ubuntu"), Some(15), "");
        assert_eq!(
            error,
            "WSL agent TERM command failed (exit=15, distro=Ubuntu)"
        );
    }

    #[test]
    fn format_wsl_signal_command_error_includes_sanitized_stderr() {
        let error =
            format_wsl_signal_command_error("KILL", None, Some(2), "kill: failed to parse pid");
        assert_eq!(
            error,
            "WSL agent KILL command failed (exit=2, distro=default): kill: failed to parse pid"
        );
    }
}
