// Path: crates/im_agent/src/repos/import/mod.rs
// Description: Copying external OS files and folders into one directory of a repo worktree

//! The caller owns the per-worktree mutation lock. Validate the entire plan before
//! writing; later failures report landed files for watcher/status reconciliation.

mod copy;
mod sources;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_git_control;
#[cfg(test)]
mod tests_refusals;
#[cfg(test)]
mod tests_support;
mod translate;

use std::path::{Path, PathBuf};

use crate::error::{AgentError, MutationEffect};
use crate::protocol::{ImportConflictPolicy, ImportedFile};
use crate::repos::normalize_directory_path;
use crate::repos::worktree::{
    ensure_distinct_destinations, normalize_authorization, resolve_destination,
};
use crate::source_control::ensure_no_git_component;
use crate::staging::{StageFileCancelToken, StagingRootKind};

pub use sources::MAX_IMPORT_ENTRIES;

/// Copies absolute host paths into a repo-relative directory in the agent's namespace.
/// The caller must hold the worktree's mutation lock; results list landed files in order.
pub async fn import_files(
    repo_root: &Path,
    directory: &str,
    sources: &[String],
    policy: &ImportConflictPolicy,
    staging_kind: StagingRootKind,
    cancel: &StageFileCancelToken,
) -> Result<Vec<ImportedFile>, AgentError> {
    match translate::translate_sources(sources, staging_kind) {
        Ok(sources) => import_resolved(repo_root, directory, sources, policy, cancel).await,
        Err(error) => Err(error.with_default_effect(MutationEffect::NotApplied)),
    }
}

/// Imports sources already resolved in the agent's namespace, including in-repo copies.
pub(crate) async fn import_resolved(
    repo_root: &Path,
    directory: &str,
    sources: Vec<PathBuf>,
    policy: &ImportConflictPolicy,
    cancel: &StageFileCancelToken,
) -> Result<Vec<ImportedFile>, AgentError> {
    import_inner(repo_root, directory, &sources, policy, cancel)
        .await
        .map_err(|error| error.with_default_effect(MutationEffect::NotApplied))
}

async fn import_inner(
    repo_root: &Path,
    directory: &str,
    sources: &[PathBuf],
    policy: &ImportConflictPolicy,
    cancel: &StageFileCancelToken,
) -> Result<Vec<ImportedFile>, AgentError> {
    let directory = normalize_directory_path(directory)?;
    ensure_no_git_component(&directory)?;
    let policy = normalize_authorization(policy)?;
    let destination = resolve_destination(repo_root, &directory).await?;

    let planned = sources::plan_sources(sources, &directory, &destination, cancel).await?;
    ensure_distinct_destinations(planned.iter().map(|source| source.dest_rel.as_str()))?;
    copy::ensure_writable(repo_root, &planned, &policy).await?;

    copy::write_planned(repo_root, &planned, &policy, cancel).await
}

fn unsupported_source(source: impl std::fmt::Display, reason: &str) -> AgentError {
    AgentError::new(
        "IMPORT_UNSUPPORTED_SOURCE",
        format!("Cannot import {source}: it {reason}"),
    )
}

/// The wire has no cancelled variant; report landed files and their reconciliation effect.
fn cancelled_error(imported: &[ImportedFile]) -> AgentError {
    AgentError::internal("Import cancelled before it finished")
        .with_details(serde_json::json!({ "imported": imported }))
        .with_effect(if imported.is_empty() {
            MutationEffect::NotApplied
        } else {
            MutationEffect::Unknown
        })
}
