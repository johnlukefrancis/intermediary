// Path: crates/im_agent/src/protocol/commands_source_control.rs
// Description: UI-to-agent source-control command payloads (status, diff, tagged actions)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlStatusCommand {
    pub repo_id: String,
}

/// Which side of the index a diff or entry refers to. `Index` compares HEAD to
/// the index (staged), `Worktree` compares the index to the working tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceControlArea {
    Index,
    Worktree,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlDiffCommand {
    pub repo_id: String,
    pub path: String,
    /// Rename source for renamed/copied entries so the diff can pair both paths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_path: Option<String>,
    pub area: SourceControlArea,
}

/// Pathspec scope for stage/unstage. `All` means everything under the configured
/// repo root (`-- .`), never the whole repository. `Paths` with an empty list is
/// rejected by the agent before any process spawns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum SourceControlScope {
    All,
    Paths { paths: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SourceControlActionPayload {
    Stage { scope: SourceControlScope },
    Unstage { scope: SourceControlScope },
    Discard { paths: Vec<String> },
    Commit { message: String },
    Push,
    Pull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceControlActionKind {
    Stage,
    Unstage,
    Discard,
    Commit,
    Push,
    Pull,
}

impl SourceControlActionPayload {
    pub fn kind(&self) -> SourceControlActionKind {
        match self {
            Self::Stage { .. } => SourceControlActionKind::Stage,
            Self::Unstage { .. } => SourceControlActionKind::Unstage,
            Self::Discard { .. } => SourceControlActionKind::Discard,
            Self::Commit { .. } => SourceControlActionKind::Commit,
            Self::Push => SourceControlActionKind::Push,
            Self::Pull => SourceControlActionKind::Pull,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlActionCommand {
    pub repo_id: String,
    pub action: SourceControlActionPayload,
}
