// Path: crates/im_agent/src/repos/worktree/mod.rs
// Description: The four worktree entry actions (delete, move, copy, rename) behind one caller-locked owner

//! A worktree action is a repo mutation that never speaks to Git: it moves,
//! copies, renames, or removes entries in the worktree and lets the watcher
//! and the next status read describe what changed. It therefore owes the same
//! guarantees a Git mutation does, and the caller holds the same per-worktree
//! lock (`SourceControlLocks::acquire`) before calling in — this module never
//! takes it, so one owner decides both admission and serialization.
//!
//! Every arm follows one order: validate every entry, refuse the whole action,
//! and only then write. An error raised before the first write proves the
//! worktree is untouched (`effect: notApplied`); from there on the error
//! carries what landed in `details.applied` and reports `unknown`, because a
//! half-applied action is exactly the state only a fresh read can describe.
//!
//! Deleting is the one arm that does not live here: removal is recoverable by
//! construction, and the machinery that makes it so — the per-operation
//! quarantine, its markers and its startup sweep — belongs to source control.
//! This module routes to it rather than growing a second one.

mod copy_entries;
mod destination;
mod entries;
mod move_entries;
mod rename;
#[cfg(test)]
mod tests_copy;
#[cfg(test)]
mod tests_move;
#[cfg(test)]
mod tests_no_replace;
#[cfg(test)]
mod tests_rename;
#[cfg(test)]
mod tests_support;

use std::path::Path;

use crate::error::{AgentError, MutationEffect};
use crate::protocol::WorktreeAction;
use crate::source_control::{quarantine_entries, SourceControlLocks};
use crate::staging::StageFileCancelToken;

/// The destination law an import shares with every in-worktree write: both
/// land paths in a repo directory without Git, so both owe the same proofs
/// about where a relative path resolves and which paths one action may claim.
pub(crate) use destination::{
    ensure_distinct_destinations, join_relative, normalize_authorization, resolve_destination,
};
pub(crate) use entries::{conflict_error, existing_kind, kind_mismatch_error};

/// Runs one worktree action and returns the repo-relative paths it produced.
///
/// The caller must already hold this worktree's mutation lock; `locks` is
/// passed on so a delete can register its quarantine operation against the
/// same registry the startup sweep asks.
pub async fn worktree_action(
    repo_root: &Path,
    action: &WorktreeAction,
    locks: &SourceControlLocks,
    cancel: &StageFileCancelToken,
) -> Result<Vec<String>, AgentError> {
    let outcome = match action {
        WorktreeAction::Delete { paths } => quarantine_entries(repo_root, paths, locks).await,
        WorktreeAction::Move {
            paths,
            directory,
            on_conflict,
        } => move_entries::move_entries(repo_root, paths, directory, on_conflict).await,
        WorktreeAction::Copy {
            paths,
            directory,
            on_conflict,
        } => copy_entries::copy_entries(repo_root, paths, directory, on_conflict, cancel).await,
        WorktreeAction::Rename { path, new_name } => {
            rename::rename_entry(repo_root, path, new_name)
                .await
                .map(|renamed| vec![renamed])
        }
    };
    outcome.map_err(|error| error.with_default_effect(MutationEffect::NotApplied))
}
