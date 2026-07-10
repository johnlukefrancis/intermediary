// Path: src-tauri/src/lib/obs/logging.rs
// Description: File-based logger writing to run_latest.txt

use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::panic::{self, AssertUnwindSafe, PanicHookInfo};
use std::path::{Path, PathBuf};
use std::sync::{Once, OnceLock};

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static PANIC_HOOK_INIT: Once = Once::new();
const PANIC_DETAIL_LIMIT_CHARS: usize = 2_048;

pub fn init_before_tauri(app_identifier: &str) {
    if LOG_PATH.get().is_some() {
        return;
    }
    let configured_log_dir = std::env::var("INTERMEDIARY_LOG_DIR")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from);
    let fallback_log_dir =
        dirs::data_local_dir().map(|base| base.join(app_identifier).join("logs"));

    if let Some(log_file) =
        prepare_first_usable_log_file(configured_log_dir.as_deref(), fallback_log_dir.as_deref())
    {
        let _ = LOG_PATH.set(log_file);
        return;
    }

    eprintln!("Failed to initialize Intermediary logging before Tauri startup");
}

pub fn install_panic_hook() {
    PANIC_HOOK_INIT.call_once(|| {
        let previous_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            let _ = panic::catch_unwind(AssertUnwindSafe(|| {
                log(
                    "error",
                    "panic",
                    "unhandled",
                    &format_panic_details(panic_info),
                );
            }));
            previous_hook(panic_info);
        }));
    });
}

fn format_panic_details(panic_info: &PanicHookInfo<'_>) -> String {
    let payload = panic_info
        .payload()
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| {
            panic_info
                .payload()
                .downcast_ref::<String>()
                .map(String::as_str)
        })
        .unwrap_or("non-string panic payload");
    let location = panic_info
        .location()
        .map(|location| {
            format!(
                "{}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            )
        })
        .unwrap_or_else(|| "unknown".to_string());
    format!(
        "payload={} location={}",
        sanitize_log_value(payload),
        sanitize_log_value(&location)
    )
}

fn sanitize_log_value(value: &str) -> String {
    let mut sanitized = String::new();
    let mut chars = value.chars();
    for character in chars.by_ref().take(PANIC_DETAIL_LIMIT_CHARS) {
        sanitized.push(match character {
            '\r' | '\n' | '\t' => ' ',
            value if value.is_control() => '�',
            value => value,
        });
    }
    if chars.next().is_some() {
        sanitized.push_str("…[truncated]");
    }
    sanitized
}

/// Initialize the logger with the given log directory.
pub fn init(log_dir: &Path) -> bool {
    if LOG_PATH.get().is_some() {
        return true;
    }

    let log_file = match prepare_log_file(log_dir) {
        Ok(log_file) => log_file,
        Err(error) => {
            eprintln!(
                "Failed to initialize log directory {}: {error}",
                log_dir.display()
            );
            return false;
        }
    };
    LOG_PATH.set(log_file).is_ok() || LOG_PATH.get().is_some()
}

fn prepare_log_file(log_dir: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(log_dir)?;
    let log_file = log_dir.join("run_latest.txt");
    fs::write(&log_file, "")?;
    Ok(log_file)
}

fn prepare_first_usable_log_file(
    configured_log_dir: Option<&Path>,
    fallback_log_dir: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(log_dir) = configured_log_dir {
        match prepare_log_file(log_dir) {
            Ok(log_file) => return Some(log_file),
            Err(error) => {
                eprintln!(
                    "Failed to initialize configured log directory {}: {error}",
                    log_dir.display()
                );
                eprintln!(
                    "Configured Intermediary log directory is unusable; attempting app-local fallback"
                );
            }
        }
    }

    let fallback_log_dir = fallback_log_dir.filter(|path| Some(*path) != configured_log_dir)?;
    match prepare_log_file(fallback_log_dir) {
        Ok(log_file) => Some(log_file),
        Err(error) => {
            eprintln!(
                "Failed to initialize app-local log directory {}: {error}",
                fallback_log_dir.display()
            );
            None
        }
    }
}

/// Write a log entry
///
/// Format: `[2024-01-15 14:32:01] LEVEL [scope] event details`
pub fn log(level: &str, scope: &str, event: &str, details: &str) {
    let Some(log_path) = LOG_PATH.get() else {
        return;
    };

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let level_upper = level.to_uppercase();
    let line = format!("[{timestamp}] {level_upper} [{scope}] {event} {details}\n");

    let result = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .and_then(|mut f| f.write_all(line.as_bytes()));

    if let Err(e) = result {
        eprintln!("Failed to write log: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::{prepare_first_usable_log_file, sanitize_log_value};
    use tempfile::tempdir;

    #[test]
    fn panic_log_values_are_single_line_and_bounded() {
        let oversized = format!("line one\nline two\t{}", "x".repeat(2_100));
        let sanitized = sanitize_log_value(&oversized);

        assert!(!sanitized.contains('\n'));
        assert!(!sanitized.contains('\t'));
        assert!(sanitized.ends_with("…[truncated]"));
    }

    #[test]
    fn unusable_configured_directory_can_fall_back_to_app_local_storage() {
        let root = tempdir().expect("tempdir");
        let blocking_file = root.path().join("not-a-directory");
        std::fs::write(&blocking_file, "blocking file").expect("blocking file");
        let configured = blocking_file.join("logs");
        let fallback = root.path().join("app-local/logs");

        let log_file = prepare_first_usable_log_file(Some(&configured), Some(&fallback))
            .expect("fallback log file");

        assert_eq!(log_file, fallback.join("run_latest.txt"));
        assert!(log_file.is_file());
    }
}
