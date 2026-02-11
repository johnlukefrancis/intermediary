// Path: src-tauri/src/lib/agent/process_control.rs
// Description: Spawn helpers for host/WSL agents and readiness probing

use super::install::AgentBundlePaths;
use crate::commands::agent_probe::probe_port_blocking;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::io::ErrorKind;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const READY_TIMEOUT: Duration = Duration::from_secs(30);
const READY_POLL: Duration = Duration::from_millis(250);
const EARLY_EXIT_STREAM_LIMIT_BYTES: usize = 8 * 1024;

pub fn spawn_host_agent_process(
    bundle: &AgentBundlePaths,
    host_port: u16,
    wsl_port: u16,
    host_ws_token: &str,
    wsl_ws_token: &str,
    host_ws_allowed_origins: &[String],
) -> Result<Child, String> {
    if !bundle.host_agent_binary_host.is_file() {
        return Err(format!(
            "Host agent binary is missing: {}",
            bundle.host_agent_binary_host.display()
        ));
    }

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
        .spawn()
        .map_err(|err| format_host_spawn_error(&bundle.host_agent_binary_host, err))
}

pub fn wait_for_agent_ready(
    child: &mut Child,
    port: u16,
    label: &str,
    capture_output_on_early_exit: bool,
) -> Result<(), String> {
    let start = Instant::now();
    let mut last_error: Option<String> = None;

    while start.elapsed() < READY_TIMEOUT {
        if let Some(status) = child
            .try_wait()
            .map_err(|err| format!("Failed to poll {label} process: {err}"))?
        {
            let detail = if capture_output_on_early_exit {
                format_early_exit_output(child)
            } else {
                None
            };
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

fn format_early_exit_output(child: &mut Child) -> Option<String> {
    let stderr = read_stream_limited(child.stderr.take(), EARLY_EXIT_STREAM_LIMIT_BYTES);
    let stdout = read_stream_limited(child.stdout.take(), EARLY_EXIT_STREAM_LIMIT_BYTES);

    let mut sections: Vec<String> = Vec::new();
    if let Some(stderr) = stderr {
        sections.push(format_stream("stderr", stderr));
    }
    if let Some(stdout) = stdout {
        sections.push(format_stream("stdout", stdout));
    }

    if sections.is_empty() {
        None
    } else {
        Some(sections.join("; "))
    }
}

#[derive(Debug)]
struct StreamSnippet {
    text: String,
    truncated: bool,
}

fn read_stream_limited<R: Read>(stream: Option<R>, limit: usize) -> Option<StreamSnippet> {
    let stream = stream?;
    let mut reader = stream.take((limit + 1) as u64);
    let mut bytes: Vec<u8> = Vec::new();
    if reader.read_to_end(&mut bytes).is_err() {
        return Some(StreamSnippet {
            text: "<failed to read stream>".to_string(),
            truncated: false,
        });
    }

    let truncated = bytes.len() > limit;
    if truncated {
        bytes.truncate(limit);
    }

    let text = sanitize_stream_text(&String::from_utf8_lossy(&bytes));
    if text.is_empty() {
        return None;
    }

    Some(StreamSnippet { text, truncated })
}

fn sanitize_stream_text(text: &str) -> String {
    text.trim()
        .replace('\r', "")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

fn format_stream(label: &str, snippet: StreamSnippet) -> String {
    if snippet.truncated {
        return format!("{label}=\"{}\" (truncated)", snippet.text);
    }
    format!("{label}=\"{}\"", snippet.text)
}
