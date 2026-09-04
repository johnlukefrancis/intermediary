// Path: crates/im_host_agent/src/runtime/host_runtime/host_dispatch.rs
// Description: Host-rooted repo command dispatch onto the local backend for HostRuntime

use im_agent::error::AgentError;
use im_agent::protocol::{UiCommand, UiResponse};
use im_agent::server::EventBus;

use super::HostRuntime;

impl HostRuntime {
    pub(super) async fn dispatch_host_command(
        &mut self,
        command: UiCommand,
        event_bus: &EventBus,
    ) -> Result<UiResponse, AgentError> {
        match command {
            UiCommand::WatchRepo(command) => {
                let result = self
                    .local_backend
                    .watch_repo(command, event_bus, &self.logger)
                    .await?;
                Ok(UiResponse::WatchRepoResult(result))
            }
            UiCommand::Refresh(command) => {
                let result = self.local_backend.refresh_repo(command).await?;
                Ok(UiResponse::RefreshResult(result))
            }
            UiCommand::StageFile(command) => {
                let result = self.local_backend.stage_file(command).await?;
                Ok(UiResponse::StageFileResult(result))
            }
            UiCommand::ReadTextFile(command) => {
                let result = self.local_backend.read_text_file(command).await?;
                Ok(UiResponse::ReadTextFileResult(result))
            }
            UiCommand::ReadImageFile(command) => {
                let result = self.local_backend.read_image_file(command).await?;
                Ok(UiResponse::ReadImageFileResult(result))
            }
            UiCommand::BuildBundle(command) => {
                self.local_backend
                    .build_bundle(command, event_bus, &self.logger)
                    .await
            }
            UiCommand::CancelBundleBuild(command) => {
                let result = self.local_backend.cancel_bundle_build(command);
                Ok(UiResponse::CancelBundleBuildResult(result))
            }
            UiCommand::GetRepoTopLevel(command) => {
                let result = self.local_backend.get_repo_top_level(command).await?;
                Ok(UiResponse::GetRepoTopLevelResult(result))
            }
            UiCommand::ListRepoDirectory(command) => {
                let result = self.local_backend.list_repo_directory(command).await?;
                Ok(UiResponse::ListRepoDirectoryResult(result))
            }
            UiCommand::ListBundles(command) => {
                let result = self.local_backend.list_bundles(command).await?;
                Ok(UiResponse::ListBundlesResult(result))
            }
            // Source-control commands, imports, worktree actions and shutdown
            // are dispatched by the server without holding the runtime write
            // lock (a long push, a large copy or a drain must never freeze
            // every repo); reaching them here is a routing defect, not a
            // supported path.
            UiCommand::SourceControlStatus(_)
            | UiCommand::SourceControlDiff(_)
            | UiCommand::SourceControlImageDiff(_)
            | UiCommand::SourceControlAction(_)
            | UiCommand::ImportFiles(_)
            | UiCommand::WorktreeAction(_)
            | UiCommand::Shutdown => Err(AgentError::internal(
                "Source-control, import, worktree and shutdown commands must be dispatched without the runtime lock",
            )),
            UiCommand::ClientHello(_)
            | UiCommand::SetOptions(_)
            | UiCommand::GetTrFleetStatus(_)
            | UiCommand::TrFleetAction(_)
            | UiCommand::Unknown => Err(AgentError::new("UNKNOWN_COMMAND", "Unsupported command")),
        }
    }
}
