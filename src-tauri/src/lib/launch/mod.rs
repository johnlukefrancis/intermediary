// Path: src-tauri/src/lib/launch/mod.rs
// Description: Project launch targets: top-level candidate discovery, usage ranking and launch-or-focus

mod candidates;
mod launch_action;
mod recent_items;
mod shell_link;
mod unreal_engine;
mod unreal_focus;
mod unreal_recent;

use serde::Serialize;

pub use candidates::discover_candidates;
pub use launch_action::{launch_target, LaunchOutcome};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LaunchKind {
    Unreal,
    Html,
}

/// A launchable top-level file of a repository root, ordered best first by discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchCandidate {
    pub relative_path: String,
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
