// Path: src-tauri/src/lib/commands/notes.rs
// Description: Tauri commands for per-repo plain-text notes persistence

use crate::obs::logging;
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Manager};

const MAX_NOTE_BYTES: usize = 100_000;
const MAX_NOTE_PREFIX_CHARS: usize = 48;
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Resolve the notes directory under app local data, creating it if needed.
fn resolve_notes_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_local_data = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "Could not resolve app local data directory".to_string())?;
    let notes_dir = app_local_data.join("notes");
    fs::create_dir_all(&notes_dir)
        .map_err(|e| format!("Failed to create notes directory: {e}"))?;
    Ok(notes_dir)
}

fn log_notes_error(event: &str, message: String) -> String {
    logging::log("error", "notes", event, &message);
    message
}

fn sanitize_repo_id_prefix(repo_id: &str) -> String {
    let mut safe: String = repo_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    if safe.len() > MAX_NOTE_PREFIX_CHARS {
        safe.truncate(MAX_NOTE_PREFIX_CHARS);
    }
    if safe.is_empty() {
        return "repo".to_string();
    }
    safe
}

fn hash_repo_id(repo_id: &str) -> String {
    // Stable 128-bit hash (two FNV-1a style lanes) to keep filename derivation deterministic.
    let mut h1: u64 = 0xcbf29ce484222325;
    let mut h2: u64 = 0x84222325cbf29ce4;
    for byte in repo_id.as_bytes() {
        h1 ^= *byte as u64;
        h1 = h1.wrapping_mul(0x100000001b3);

        h2 ^= (*byte as u64).wrapping_add(0x9e3779b97f4a7c15);
        h2 = h2.wrapping_mul(0x100000001b3);
    }
    format!("{h1:016x}{h2:016x}")
}

fn build_note_file_stem(repo_id: &str) -> Result<String, String> {
    if repo_id.trim().is_empty() {
        return Err("repo_id must not be empty".to_string());
    }
    let safe_prefix = sanitize_repo_id_prefix(repo_id);
    let hash = hash_repo_id(repo_id);
    Ok(format!("{safe_prefix}__{hash}"))
}

fn replace_file_cross_platform(temp_path: &Path, final_path: &Path) -> io::Result<()> {
    match fs::rename(temp_path, final_path) {
        Ok(()) => Ok(()),
        Err(err)
            if err.kind() == io::ErrorKind::AlreadyExists
                || err.kind() == io::ErrorKind::PermissionDenied =>
        {
            fs::remove_file(final_path)?;
            fs::rename(temp_path, final_path)
        }
        Err(err) => Err(err),
    }
}

fn write_note_atomic(path: &Path, content: &str) -> Result<(), String> {
    let temp_suffix = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_path = path.with_extension(format!("txt.tmp.{temp_suffix}"));

    let write_result: io::Result<()> = (|| {
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        drop(file);
        replace_file_cross_platform(&temp_path, path)
    })();

    if let Err(err) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(log_notes_error(
            "save_failed",
            format!("Failed to write note file atomically: {err}"),
        ));
    }

    Ok(())
}

fn resolve_note_path(app: &AppHandle, repo_id: &str) -> Result<PathBuf, String> {
    let dir = resolve_notes_dir(app)?;
    let file_stem = build_note_file_stem(repo_id)?;
    Ok(dir.join(format!("{file_stem}.txt")))
}

/// Load a per-repo note from disk. Returns empty string if no note exists yet.
#[tauri::command]
pub async fn load_note(app: AppHandle, repo_id: String) -> Result<String, String> {
    let path = resolve_note_path(&app, &repo_id)?;

    tauri::async_runtime::spawn_blocking(move || {
        match fs::read_to_string(&path) {
            Ok(text) => Ok(text),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(String::new()),
            Err(err) => Err(log_notes_error(
                "load_failed",
                format!("Failed to read note: {err}"),
            )),
        }
    })
    .await
    .map_err(|err| {
        log_notes_error(
            "load_failed",
            format!("Note load task failed: {err}"),
        )
    })?
}

/// Save a per-repo note to disk. Atomic write (temp + sync + rename).
#[tauri::command]
pub async fn save_note(app: AppHandle, repo_id: String, content: String) -> Result<(), String> {
    if content.as_bytes().len() > MAX_NOTE_BYTES {
        return Err(format!(
            "Note exceeds maximum size ({MAX_NOTE_BYTES} bytes)"
        ));
    }

    let path = resolve_note_path(&app, &repo_id)?;

    tauri::async_runtime::spawn_blocking(move || write_note_atomic(&path, &content))
        .await
        .map_err(|err| {
            log_notes_error(
                "save_failed",
                format!("Note save task failed: {err}"),
            )
        })?
}

/// Delete a per-repo note file (used during repo removal / reset).
#[tauri::command]
pub async fn delete_note(app: AppHandle, repo_id: String) -> Result<(), String> {
    let path = resolve_note_path(&app, &repo_id)?;

    tauri::async_runtime::spawn_blocking(move || {
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(log_notes_error(
                "delete_failed",
                format!("Failed to delete note: {err}"),
            )),
        }
    })
    .await
    .map_err(|err| {
        log_notes_error(
            "delete_failed",
            format!("Note delete task failed: {err}"),
        )
    })?
}

#[cfg(test)]
mod tests {
    use super::build_note_file_stem;

    #[test]
    fn note_stem_rejects_blank_repo_id() {
        let result = build_note_file_stem("   ");
        assert!(result.is_err());
    }

    #[test]
    fn note_stem_is_collision_safe_for_sanitized_equivalents() {
        let left = build_note_file_stem("repo/a").expect("left stem");
        let right = build_note_file_stem("repo_a").expect("right stem");
        assert_ne!(left, right);
    }
}
