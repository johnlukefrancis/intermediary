// Path: crates/im_host_agent/src/runtime/local_host_worktree_backend.rs
// Description: Host-native worktree entry actions that never hold the runtime lock across the write

use std::path::Path;

use im_agent::error::AgentError;
use im_agent::protocol::{UiResponse, WorktreeActionCommand, WorktreeActionResult};
use im_agent::repos::worktree::worktree_action;
use im_agent::staging::StageFileCancelToken;

use crate::runtime::local_host_source_control_backend::HostSourceControlContext;

/// Deletes, moves, copies, or renames entries in a host-rooted repo worktree.
///
/// The write bypasses Git, so it holds the same per-worktree mutation lock a
/// commit or a discard holds — one lock decides who may write this index's
/// worktree, whichever route the write arrives by — and inherits the drain
/// gate with it. Host actions carry no cancel token: the host agent has no
/// cooperative cancellation path, and the forwarding timeout above is what
/// bounds the request.
pub async fn execute_host_worktree_action(
    command: WorktreeActionCommand,
    context: HostSourceControlContext,
) -> Result<UiResponse, AgentError> {
    let repo_root = Path::new(&context.repo_root);
    let _guard = context.locks.acquire(repo_root).await?;

    let kind = command.action.kind();
    let entries = worktree_action(
        repo_root,
        &command.action,
        &context.locks,
        &StageFileCancelToken::new(),
    )
    .await?;

    Ok(UiResponse::WorktreeActionResult(WorktreeActionResult {
        repo_id: command.repo_id,
        kind,
        entries,
    }))
}
