// Path: src-tauri/src/lib/agent/process_control.rs
// Description: Spawn helpers for host/WSL agents and readiness probing

use super::install::AgentBundlePaths;
use crate::commands::agent_probe::probe_port_blocking;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::io::ErrorKind;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const READY_TIMEOUT: Duration = Duration::from_secs(30);
const READY_POLL: Duration = Duration::from_millis(250);
const EARLY_EXIT_LOG_LIMIT_BYTES: usize = 8 * 1024;
const EARLY_EXIT_LOG_LINE_LIMIT: usize = 40;

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
        .env("INTERMEDIARY_AGENT_STDIO_LOGGING", "0")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| format_host_spawn_error(&bundle.host_agent_binary_host, err))
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

fn format_early_exit_log(log_file: &Path, log_offset: u64) -> Option<String> {
    let tail = match read_log_tail_since(
        log_file,
        log_offset,
        EARLY_EXIT_LOG_LIMIT_BYTES,
        EARLY_EXIT_LOG_LINE_LIMIT,
    ) {
        Ok(value) => value,
        Err(err) => {
            return Some(format!(
                "recent_log_unavailable path={} error={err}",
                log_file.display()
            ));
        }
    };

    let sanitized = sanitize_stream_text(&tail);
    if sanitized.is_empty() {
        return Some(format!("recent_log_empty path={}", log_file.display()));
    }

    Some(format!("recent_log={sanitized}"))
}

fn read_log_tail_since(
    log_file: &Path,
    log_offset: u64,
    limit_bytes: usize,
    line_limit: usize,
) -> Result<String, String> {
    let mut reader = std::fs::File::open(log_file)
        .map_err(|err| format!("failed to open log file: {err}"))?;
    let file_len = reader
        .metadata()
        .map_err(|err| format!("failed to read log metadata: {err}"))?
        .len();
    let clamped_offset = if log_offset > file_len { 0 } else { log_offset };
    if file_len <= clamped_offset {
        return Ok(String::new());
    }

    let start = std::cmp::max(clamped_offset, file_len.saturating_sub(limit_bytes as u64));
    let mut starts_mid_line = false;
    if start > 0 {
        reader
            .seek(SeekFrom::Start(start - 1))
            .map_err(|err| format!("failed to seek log file: {err}"))?;
        let mut prev = [0_u8; 1];
        reader
            .read_exact(&mut prev)
            .map_err(|err| format!("failed to read log file: {err}"))?;
        starts_mid_line = prev[0] != b'\n';
    }

    reader
        .seek(SeekFrom::Start(start))
        .map_err(|err| format!("failed to seek log file: {err}"))?;

    let mut bytes: Vec<u8> = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|err| format!("failed to read log file: {err}"))?;

    let slice = if starts_mid_line {
        match bytes.iter().position(|byte| *byte == b'\n') {
            Some(pos) => &bytes[(pos + 1)..],
            None => &bytes[..],
        }
    } else {
        &bytes[..]
    };
    let text = String::from_utf8_lossy(slice).into_owned();
    Ok(take_last_lines(&text, line_limit))
}

fn take_last_lines(text: &str, line_limit: usize) -> String {
    if line_limit == 0 {
        return String::new();
    }

    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= line_limit {
        return lines.join("\n");
    }
    lines[lines.len() - line_limit..].join("\n")
}

fn sanitize_stream_text(text: &str) -> String {
    text.trim()
        .replace('\r', "")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::{capture_log_cursor, read_log_tail_since};
    use std::fs::OpenOptions;
    use std::io::Write;

    #[test]
    fn read_log_tail_since_ignores_pre_spawn_lines() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log_file = dir.path().join("agent_latest.log");
        append_lines(&log_file, &["old-1", "old-2"]);
        let cursor = capture_log_cursor(&log_file);

        append_lines(&log_file, &["new-1", "new-2"]);
        let tail = read_log_tail_since(&log_file, cursor, 8 * 1024, 40).expect("tail");

        assert!(!tail.contains("old-1"));
        assert!(!tail.contains("old-2"));
        assert!(tail.contains("new-1"));
        assert!(tail.contains("new-2"));
    }

    #[test]
    fn read_log_tail_since_returns_empty_without_new_bytes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log_file = dir.path().join("agent_latest.log");
        append_lines(&log_file, &["only-old"]);
        let cursor = capture_log_cursor(&log_file);

        let tail = read_log_tail_since(&log_file, cursor, 8 * 1024, 40).expect("tail");
        assert!(tail.is_empty());
    }

    fn append_lines(log_file: &std::path::Path, lines: &[&str]) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
            .expect("open");
        for line in lines {
            writeln!(file, "{line}").expect("write");
        }
        file.flush().expect("flush");
    }
}
