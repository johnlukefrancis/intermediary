// Path: crates/im_host_agent/src/runtime/local_host_source_control_backend.rs
// Description: Host-native source-control execution that never holds the runtime lock across Git

use std::path::Path;

use im_agent::error::AgentError;
use im_agent::protocol::{
    SourceControlActionResult, SourceControlDiffResult, SourceControlStatusResult, UiCommand,
    UiResponse,
};
use im_agent::source_control::{
    run_source_control_action, source_control_diff, source_control_status, SourceControlLocks,
};

use crate::runtime::local_host_backend::LocalHostBackend;

/// Everything a host-rooted source-control command needs, cloned out of the
/// runtime under a short read lock so Git runs with no runtime lock held.
#[derive(Clone)]
pub struct HostSourceControlContext {
    pub repo_root: String,
    pub locks: SourceControlLocks,
}

impl LocalHostBackend {
    pub fn source_control_context(
        &self,
        repo_id: &str,
    ) -> Result<HostSourceControlContext, AgentError> {
        Ok(HostSourceControlContext {
            repo_root: self.host_repo_root(repo_id)?.to_string(),
            locks: self.source_control_locks(),
        })
    }
}

/// Executes a source-control command for a host-rooted repo. Host reads carry
/// no cancel token today (the host agent has no cooperative cancellation path);
/// they are bounded by their Git timeout.
pub async fn execute_host_source_control(
    command: UiCommand,
    context: HostSourceControlContext,
) -> Result<UiResponse, AgentError> {
    let repo_root = Path::new(&context.repo_root);
    match command {
        UiCommand::SourceControlStatus(command) => {
            let status = source_control_status(repo_root, None).await?;
            Ok(UiResponse::SourceControlStatusResult(
                SourceControlStatusResult {
                    repo_id: command.repo_id,
                    status,
                },
            ))
        }
        UiCommand::SourceControlDiff(command) => {
            let diff = source_control_diff(
                repo_root,
                &command.path,
                command.original_path.as_deref(),
                command.area,
                None,
            ).await?;
            Ok(UiResponse::SourceControlDiffResult(SourceControlDiffResult {
                repo_id: command.repo_id,
                path: command.path,
                area: command.area,
                patch: diff.patch,
                truncated: diff.truncated,
                binary: diff.binary,
            }))
        }
        UiCommand::SourceControlAction(command) => {
            let kind = command.action.kind();
            let outcome = run_source_control_action(
                &context.locks,
                &command.repo_id,
                repo_root,
                command.action,
            )
            .await?;
            Ok(UiResponse::SourceControlActionResult(
                SourceControlActionResult {
                    repo_id: command.repo_id,
                    kind,
                    status: outcome.status,
                    commit_sha: outcome.commit_sha,
                },
            ))
        }
        other => Err(AgentError::internal(format!(
            "{} is not a source-control command",
            other.command_type()
        ))),
    }
}
