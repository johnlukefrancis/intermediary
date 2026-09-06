// Path: crates/im_host_agent/src/runtime/local_host_repo_backend.rs
// Description: Host-native repo read and topology operations

use im_agent::error::AgentError;
use im_agent::protocol::{
    GetRepoTopLevelCommand, GetRepoTopLevelResult, ListRepoDirectoryCommand,
    ListRepoDirectoryResult, ReadImageFileCommand, ReadImageFileResult, ReadTextFileCommand,
    ReadTextFileResult,
};
use im_agent::repos::{
    get_repo_top_level, list_repo_directory, read_image_file_bounded, read_text_file,
};

use crate::runtime::local_host_backend::LocalHostBackend;

impl LocalHostBackend {
    pub async fn read_text_file(
        &self,
        command: ReadTextFileCommand,
    ) -> Result<ReadTextFileResult, AgentError> {
        let repo_root = self.host_repo_root(&command.repo_id)?;
        let result = read_text_file(repo_root, &command.path).await?;

        Ok(ReadTextFileResult {
            repo_id: command.repo_id,
            path: command.path,
            content: result.content,
            bytes: result.bytes,
            mtime_ms: result.mtime_ms,
            encoding: "utf-8".to_string(),
        })
    }

    pub async fn read_image_file(
        &self,
        command: ReadImageFileCommand,
    ) -> Result<ReadImageFileResult, AgentError> {
        let repo_root = self.host_repo_root(&command.repo_id)?;
        let result = read_image_file_bounded(repo_root, &command.path, command.max_bytes).await?;

        Ok(ReadImageFileResult {
            repo_id: command.repo_id,
            path: command.path,
            data_base64: result.data_base64,
            mime_type: result.mime_type,
            bytes: result.bytes,
            mtime_ms: result.mtime_ms,
        })
    }

    pub async fn get_repo_top_level(
        &self,
        command: GetRepoTopLevelCommand,
    ) -> Result<GetRepoTopLevelResult, AgentError> {
        let repo_root = self.host_repo_root(&command.repo_id)?;

        let result = get_repo_top_level(repo_root)
            .await
            .map_err(|err| AgentError::internal(format!("Failed to scan repo: {err}")))?;

        Ok(GetRepoTopLevelResult {
            repo_id: command.repo_id,
            dirs: result.dirs,
            files: result.files,
            subdirs: Some(result.subdirs),
            default_excluded: result.default_excluded,
        })
    }

    pub async fn list_repo_directory(
        &self,
        command: ListRepoDirectoryCommand,
    ) -> Result<ListRepoDirectoryResult, AgentError> {
        let repo_root = self.host_repo_root(&command.repo_id)?;
        let result = list_repo_directory(repo_root, &command.path).await?;

        Ok(ListRepoDirectoryResult {
            repo_id: command.repo_id,
            path: result.path,
            dirs: result.dirs,
            files: result.files,
        })
    }
}
