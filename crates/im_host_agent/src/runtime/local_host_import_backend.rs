// Path: crates/im_host_agent/src/runtime/local_host_import_backend.rs
// Description: Host-native file import execution that never holds the runtime lock across the copy

use std::path::Path;

use im_agent::error::AgentError;
use im_agent::protocol::{ImportFilesCommand, ImportFilesResult, UiResponse};
use im_agent::repos::import_files;
use im_agent::staging::{StageFileCancelToken, StagingRootKind};

use crate::runtime::local_host_source_control_backend::HostSourceControlContext;

/// Copies external OS files into one directory of a host-rooted repo.
///
/// An import writes the worktree without Git, so it holds the same
/// per-worktree mutation lock a commit or a discard holds — one lock decides
/// who may write this index's worktree, whichever route the write arrives by —
/// and inherits the drain gate with it. Host imports carry no cancel token:
/// the host agent has no cooperative cancellation path, and the forwarding
/// timeout above is what bounds the request.
pub async fn execute_host_import(
    command: ImportFilesCommand,
    context: HostSourceControlContext,
) -> Result<UiResponse, AgentError> {
    let repo_root = Path::new(&context.repo_root);
    let _guard = context.locks.acquire(repo_root).await?;

    let imported = import_files(
        repo_root,
        &command.directory,
        &command.sources,
        &command.on_conflict,
        StagingRootKind::Host,
        &StageFileCancelToken::new(),
    )
    .await?;

    Ok(UiResponse::ImportFilesResult(ImportFilesResult {
        repo_id: command.repo_id,
        directory: command.directory,
        imported,
    }))
}
