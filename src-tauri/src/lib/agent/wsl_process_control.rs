// Path: src-tauri/src/lib/agent/wsl_process_control.rs
// Description: WSL agent launch target resolution and spawning

use super::install::AgentBundlePaths;
use super::wsl_process_control_commands::{build_wsl_bash_args, build_wsl_spawn_command_line};
use crate::paths::wsl_convert::windows_to_wsl_path;
use crate::wsl_control::{distro_label, normalize_distro};
use std::path::Path;
use std::process::{Child, Command, Stdio};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const WSL_AGENT_BINARY_NAME: &str = "im_agent";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WslLaunchTarget {
    pub distro: Option<String>,
    pub agent_dir_wsl: String,
    pub agent_bin_wsl: String,
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

/// Starts the WSL backend with a **piped stdin the supervisor never writes to**.
/// That pipe is the agent's third shutdown owner (`im_agent::server::stdin_eof`):
/// its write end lives inside the returned [`Child`] and therefore closes exactly
/// when the supervisor stops intending this process to run — including when the
/// supervisor itself dies without a chance to send `shutdown` or SIGTERM. The
/// agent then takes the same drain path it takes for SIGTERM, so a Git mutation
/// in flight finishes and its process trees are never orphaned.
///
/// The handle is deliberately not `take()`n anywhere: moving it out would split
/// the pipe's lifetime from the process it governs, which is the one thing this
/// owner must not allow. A WSL agent this supervisor adopted rather than spawned
/// has no `Child` and therefore no pipe — logged where the adoption happens.
///
/// stdout/stderr stay null: the agent writes its own log file, and an undrained
/// pipe there would stall it (`INTERMEDIARY_AGENT_STDIO_LOGGING=0`).
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
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| format_wsl_spawn_error(err, target.distro.as_deref()))
}

pub fn format_wsl_target(target: &WslLaunchTarget) -> String {
    let distro = target.distro.as_deref().unwrap_or("default");
    format!("distro={distro} agent_bin_wsl={}", target.agent_bin_wsl)
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

#[cfg(test)]
mod tests {
    use super::format_wsl_target;
    use crate::agent::wsl_process_control::WslLaunchTarget;

    #[test]
    fn a_target_without_a_distro_is_summarised_as_the_default_one() {
        let target = WslLaunchTarget {
            distro: None,
            agent_dir_wsl: "/mnt/c/agent".to_string(),
            agent_bin_wsl: "/mnt/c/agent/im_agent".to_string(),
        };
        assert_eq!(
            format_wsl_target(&target),
            "distro=default agent_bin_wsl=/mnt/c/agent/im_agent"
        );
    }
}
