// Path: src-tauri/src/lib/agent/process_control/log_tail.rs
// Description: Reads the agent log written since a spawn cursor, bounded by bytes and lines, for early-exit reporting

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const EARLY_EXIT_LOG_LIMIT_BYTES: usize = 8 * 1024;
const EARLY_EXIT_LOG_LINE_LIMIT: usize = 40;

pub(super) fn format_early_exit_log(log_file: &Path, log_offset: u64) -> Option<String> {
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
    let mut reader =
        std::fs::File::open(log_file).map_err(|err| format!("failed to open log file: {err}"))?;
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
    use super::super::capture_log_cursor;
    use super::read_log_tail_since;
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
