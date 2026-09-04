// Path: crates/im_agent/src/repos/import/mod.rs
// Description: Copying external OS files and folders into one directory of a repo worktree

//! An import is a repo mutation that never speaks to Git: it writes files into
//! the worktree and lets the watcher and the next status read describe what
//! changed. It therefore owes the same guarantees a Git mutation does, and the
//! caller holds the same per-worktree lock (`SourceControlLocks::acquire`)
//! before calling in — this module never takes it, so one owner decides both
//! admission and serialization.
//!
//! The order below is the contract: normalize the destination and prove it
//! does not name the repository's own Git directory, prove the replace
//! authorization, resolve the destination, validate and expand every source,
//! refuse the whole drop, and only then write. Any error raised before
//! `write_planned` proves the worktree is untouched; from there on the error
//! carries what landed.

mod copy;
mod sources;
mod translate;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_refusals;
#[cfg(test)]
mod tests_support;

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

/// Copies `sources` into `<repo_root>/<directory>`.
///
/// `directory` is a repo-relative slash path (`""` or `"."` is the worktree
/// root) and `sources` are absolute OS paths in the host's namespace;
/// `staging_kind` says which namespace this agent lives in, and therefore how
/// those paths translate. The returned list names every file that landed, in
/// the order it was written.
///
/// The caller must already hold this worktree's mutation lock.
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

/// The same copy, given sources already resolved in this agent's own
/// namespace. This is what an in-repo copy hands over: its sources are
/// worktree entries the caller has already validated, so there is nothing left
/// to translate and no host path form to interpret.
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

/// Cancellation is reported as an internal failure rather than its own code:
/// the wire contract has no cancelled outcome for an import, and what the UI
/// needs is the same thing every other interruption gives it — the files that
/// landed, and an effect that makes it re-read.
fn cancelled_error(imported: &[ImportedFile]) -> AgentError {
    AgentError::internal("Import cancelled before it finished")
        .with_details(serde_json::json!({ "imported": imported }))
        .with_effect(if imported.is_empty() {
            MutationEffect::NotApplied
        } else {
            MutationEffect::Unknown
        })
}
