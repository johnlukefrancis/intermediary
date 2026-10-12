// Path: src-tauri/src/lib/launch/launch_action.rs
// Description: Launches one top-level target: focus-or-start for Unreal projects, default handler for HTML

use serde::Serialize;
use std::path::Path;
use std::process::Command;

use super::unreal_engine::resolve_editor;
use super::unreal_focus::{focus_running_editor, EditorPresence};
use super::{kind_for_file_name, path_identity, LaunchKind};
use crate::commands::file_open_policy::open_paths_with_default_app;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LaunchOutcome {
    Launched,
    Focused,
    /// The project's editor is already starting; nothing new was started.
    Starting,
}

/// Launch `file_name`, a top-level file of the host-visible `host_root`.
pub fn launch_target(host_root: &str, file_name: &str) -> Result<LaunchOutcome, String> {
    let kind = kind_for_file_name(file_name)
        .filter(|_| is_plain_file_name(file_name))
        .ok_or_else(|| format!("'{file_name}' is not a launchable top-level file"))?;
    let separator = if host_root.contains('\\') { '\\' } else { '/' };
    let target = format!(
        "{}{separator}{file_name}",
        host_root.trim_end_matches(['\\', '/'])
    );
    if !Path::new(&target).is_file() {
        return Err(format!("Launch target does not exist: {target}"));
    }
    match kind {
        LaunchKind::Unreal => launch_unreal(&target),
        LaunchKind::Html => {
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

fn is_plain_file_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\', ':']) && name != "." && name != ".."
}
