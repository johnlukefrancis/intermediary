// Path: src-tauri/src/lib/launch/launch_action.rs
// Description: Launches repo files, explicit host files and browser URLs through the platform owner

use serde::Serialize;
use std::path::Path;
use std::process::Command;

use super::target::resolve_target;
use super::unreal_engine::resolve_editor;
use super::unreal_focus::{focus_running_editor, EditorPresence};
use super::{path_identity, LaunchKind};
use crate::commands::file_open_policy::open_paths_with_default_app;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LaunchOutcome {
    Launched,
    Focused,
    /// The project's editor is already starting; nothing new was started.
    Starting,
}

/// Launch a supported target, resolving repo files against their host root.
pub fn launch_target(host_root: &str, target: &str) -> Result<LaunchOutcome, String> {
    let (target, kind) = resolve_target(host_root, target)?;
    match kind {
        LaunchKind::Unreal => launch_unreal(&target),
        LaunchKind::Html | LaunchKind::Web => {
            open_paths_with_default_app(&[target])?;
            Ok(LaunchOutcome::Launched)
        }
    }
}

fn launch_unreal(project: &str) -> Result<LaunchOutcome, String> {
    if !cfg!(windows) {
        return Err("Unreal launch is Windows-only".to_string());
    }
    match focus_running_editor(&path_identity(project)) {
        EditorPresence::Focused => return Ok(LaunchOutcome::Focused),
        EditorPresence::Starting => return Ok(LaunchOutcome::Starting),
        EditorPresence::NotRunning => {}
    }
    let editor = resolve_editor(Path::new(project))?;
    Command::new(&editor)
        .arg(project)
        .spawn()
        .map_err(|e| format!("Failed to start {}: {e}", editor.display()))?;
    Ok(LaunchOutcome::Launched)
}
