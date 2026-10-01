// Path: src-tauri/src/lib/agent/bundle_resources.rs
// Description: Resolves packaged agent resources and their version metadata

use super::install::{
    requires_wsl_binary, AGENT_BUNDLE_DIR, AGENT_VERSION_FILE, WSL_AGENT_BINARY_FILE,
};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct AgentBundleVersion {
    version: String,
}

pub(super) fn read_version(path: &Path) -> Result<String, String> {
    let contents = fs::read_to_string(path)
        .map_err(|err| format!("Failed to read agent bundle version: {err}"))?;
    let parsed: AgentBundleVersion = serde_json::from_str(&contents)
        .map_err(|err| format!("Failed to parse agent bundle version: {err}"))?;
    let trimmed = parsed.version.trim();
    if trimmed.is_empty() {
        return Err("Agent bundle version is empty".to_string());
    }
    Ok(trimmed.to_string())
}

pub(super) fn resolve_bundle_dir(resource_dir: &Path) -> Result<PathBuf, String> {
    let mut tried: Vec<PathBuf> = Vec::new();
    let mut candidates: Vec<PathBuf> = Vec::new();

    candidates.push(resource_dir.join(AGENT_BUNDLE_DIR));
    candidates.push(resource_dir.join("resources").join(AGENT_BUNDLE_DIR));

    #[cfg(any(not(target_os = "macos"), debug_assertions))]
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("resources").join(AGENT_BUNDLE_DIR));
        candidates.push(
            cwd.join("src-tauri")
                .join("resources")
                .join(AGENT_BUNDLE_DIR),
        );
    }

    #[cfg(not(target_os = "macos"))]
    if let Ok(win_root) = std::env::var("INTERMEDIARY_WIN_PATH") {
        if !win_root.trim().is_empty() {
            candidates.push(
                PathBuf::from(win_root)
                    .join("src-tauri")
                    .join("resources")
                    .join(AGENT_BUNDLE_DIR),
            );
        }
    }

    for candidate in candidates {
        tried.push(candidate.clone());
        if !candidate.is_dir() {
            continue;
        }
        if bundle_has_core_files(&candidate) {
            return Ok(candidate);
        }
    }

    let attempted = tried
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    Err(format!(
        "Agent bundle resources missing required files for this platform. Tried: {attempted}"
    ))
}

fn bundle_has_core_files(bundle_dir: &Path) -> bool {
    if !bundle_dir.join(AGENT_VERSION_FILE).is_file() {
        return false;
    }
    if requires_wsl_binary() && !bundle_dir.join(WSL_AGENT_BINARY_FILE).is_file() {
        return false;
    }
    true
}
