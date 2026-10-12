// Path: src-tauri/src/lib/launch/recent_items.rs
// Description: Windows Recent-items evidence: when each candidate file was last opened through the shell

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use super::path_identity;
use super::shell_link::link_target;

const MAX_LINK_BYTES: u64 = 64 * 1024;

fn recent_dir() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    Some(dirs::data_dir()?.join(r"Microsoft\Windows\Recent"))
}

/// Last-open times keyed by `path_identity`, read only from shortcuts named after a wanted stem.
pub fn recent_item_evidence(wanted_stems: &[String]) -> HashMap<String, i64> {
    let mut evidence = HashMap::new();
    let Some(dir) = recent_dir() else {
        return evidence;
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return evidence;
    };
    let stems: Vec<String> = wanted_stems.iter().map(|stem| stem.to_lowercase()).collect();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if !name.ends_with(".lnk") || !stems.iter().any(|stem| name.starts_with(stem.as_str())) {
            continue;
        }
        if let Some((target, opened_ms)) = read_link(&entry.path()) {
            let slot = evidence.entry(path_identity(&target)).or_insert(opened_ms);
            *slot = (*slot).max(opened_ms);
        }
    }
    evidence
}

fn read_link(path: &Path) -> Option<(String, i64)> {
    let metadata = std::fs::metadata(path).ok()?;
    if metadata.len() > MAX_LINK_BYTES {
        return None;
    }
    let modified = metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    let bytes = std::fs::read(path).ok()?;
    Some((link_target(&bytes)?, i64::try_from(modified.as_millis()).ok()?))
}
