// Path: crates/im_agent/src/protocol/responses_import.rs
// Description: Agent-to-UI response payload listing the files one import landed in the worktree

use serde::{Deserialize, Serialize};

/// One file that reached the worktree. `path` is repo-relative and
/// slash-joined, matching every other repo path on the wire; `bytes` is what
/// was copied for that file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedFile {
    pub path: String,
    pub bytes: u64,
}

/// The answer to `importFiles`: the directory the drop targeted (normalized)
/// and every file that landed under it. Directories the import created are not
/// listed; only files carry bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFilesResult {
    pub repo_id: String,
    pub directory: String,
    pub imported: Vec<ImportedFile>,
}
