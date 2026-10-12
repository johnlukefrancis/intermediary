// Path: src-tauri/src/lib/launch/mod.rs
// Description: Project launch targets: discovered repo files, explicit host files and browser URLs

mod candidates;
mod launch_action;
mod recent_items;
mod shell_link;
mod target;
mod unreal_engine;
mod unreal_focus;
mod unreal_recent;

use serde::Serialize;

pub use candidates::discover_candidates;
pub(crate) use candidates::include_configured_target;
pub use launch_action::{launch_target, LaunchOutcome};
pub(crate) use target::standalone_target;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LaunchKind {
    Unreal,
    Html,
    Web,
}

/// A discovered repo file or explicitly configured file/URL, ordered best first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchCandidate {
    pub target: String,
    pub kind: LaunchKind,
    /// Most recent observed open of this exact file, Unix milliseconds.
    pub last_used_ms: Option<i64>,
}

pub(crate) fn kind_for_file_name(name: &str) -> Option<LaunchKind> {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".uproject") {
        Some(LaunchKind::Unreal)
    } else if lower.ends_with(".html") || lower.ends_with(".htm") {
        Some(LaunchKind::Html)
    } else {
        None
    }
}

/// Case- and separator-insensitive identity for Windows paths from different recorders.
pub(crate) fn path_identity(path: &str) -> String {
    let unified = path.trim().trim_matches('"').replace('/', "\\").to_lowercase();
    match unified.strip_prefix(r"\\wsl$\") {
        Some(rest) => format!(r"\\wsl.localhost\{rest}"),
        None => unified,
    }
}
