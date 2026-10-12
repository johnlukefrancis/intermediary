// Path: src-tauri/src/lib/commands/launch.rs
// Description: Tauri commands for project launch candidates and one-click launch-or-focus

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::AppHandle;

use super::file_manager::resolve_host_path;
use super::wsl_distro::resolve_runtime_wsl_distro;
use crate::config::types::RepoRoot;
use crate::launch::{self, LaunchCandidate, LaunchOutcome};
use crate::obs::logging;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRoot {
    repo_id: String,
    root: RepoRoot,
}

/// Distro UNC prefixes, resolved once per app run: discovery reruns on every window focus.
static WSL_PREFIXES: Mutex<Vec<(Option<String>, String)>> = Mutex::new(Vec::new());

fn wsl_prefix(distro: Option<&str>) -> Option<String> {
    let mut cache = WSL_PREFIXES.lock().ok()?;
    if let Some((_, prefix)) = cache.iter().find(|(d, _)| d.as_deref() == distro) {
        return Some(prefix.clone());
    }
    let prefix = resolve_host_path("/", distro).ok()?;
    cache.push((distro.map(ToString::to_string), prefix.clone()));
    Some(prefix)
}

/// Host-visible roots; native WSL roots share one resolved distro prefix instead of one wslpath each.
fn host_roots(roots: &[RepoRoot], distro: Option<&str>) -> Vec<Option<String>> {
    roots
        .iter()
        .map(|root| match root {
            RepoRoot::Host { path } => Some(path.clone()),
            RepoRoot::Wsl { path } if path.starts_with("/mnt/") || !cfg!(windows) => {
                resolve_host_path(path, distro).ok()
            }
            RepoRoot::Wsl { path } => {
                let prefix = wsl_prefix(distro)?;
                Some(format!(
                    "{}\\{}",
                    prefix.trim_end_matches('\\'),
                    path.trim_start_matches('/').replace('/', "\\")
                ))
            }
        })
        .collect()
}

#[tauri::command]
pub async fn launch_candidates(
    app: AppHandle,
    roots: Vec<LaunchRoot>,
    distro_override: Option<String>,
) -> Result<HashMap<String, Vec<LaunchCandidate>>, String> {
    let distro = resolve_runtime_wsl_distro(&app, distro_override.as_deref());
    tauri::async_runtime::spawn_blocking(move || {
        let repo_roots: Vec<RepoRoot> = roots.iter().map(|r| r.root.clone()).collect();
        let resolved = host_roots(&repo_roots, distro.as_deref());
        let readable: Vec<String> = resolved.iter().flatten().cloned().collect();
        let mut discovered = launch::discover_candidates(&readable).into_iter();
        let mut by_repo = HashMap::new();
        for (entry, host_root) in roots.iter().zip(&resolved) {
            let candidates = match host_root {
                Some(_) => discovered.next().unwrap_or_default(),
                None => {
                    logging::log("warn", "launch", "root_unresolved", &entry.repo_id);
                    Vec::new()
                }
            };
            by_repo.insert(entry.repo_id.clone(), candidates);
        }
        by_repo
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))
}

#[tauri::command]
pub async fn launch_target(
    app: AppHandle,
    root: RepoRoot,
    relative_path: String,
    distro_override: Option<String>,
) -> Result<LaunchOutcome, String> {
    let distro = resolve_runtime_wsl_distro(&app, distro_override.as_deref());
    tauri::async_runtime::spawn_blocking(move || {
        let host_root = host_roots(std::slice::from_ref(&root), distro.as_deref())
            .pop()
            .flatten()
            .ok_or_else(|| format!("Failed to resolve repo root {}", root.path()))?;
        let outcome = launch::launch_target(&host_root, &relative_path);
        let detail = format!("{relative_path} in {host_root}: {outcome:?}");
        logging::log("info", "launch", "launch_target", &detail);
        outcome
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}
