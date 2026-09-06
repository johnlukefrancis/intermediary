// Path: crates/im_agent/src/protocol/commands.rs
// Description: UI-to-agent command payloads for the WebSocket protocol

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::commands_import::ImportFilesCommand;
use super::commands_source_control::{
    SourceControlActionCommand, SourceControlDiffCommand, SourceControlImageDiffCommand,
    SourceControlStatusCommand,
};
use super::commands_tr_fleet::{GetTrFleetStatusCommand, TrFleetActionCommand};
use super::commands_worktree::WorktreeActionCommand;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientHelloCommand {
    // TODO(protocol-precision): replace Value with typed AppConfig once shared schema exists.
    pub config: Value,
    pub staging_host_root: String,
    pub staging_wsl_root: Option<String>,
    pub auto_stage_on_change: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientHelloCommandWire {
    // TODO(protocol-precision): replace Value with typed AppConfig once shared schema exists.
    config: Value,
    #[serde(default)]
    staging_host_root: Option<String>,
    #[serde(default)]
    staging_win_root: Option<String>,
    #[serde(default)]
    staging_wsl_root: Option<String>,
    #[serde(default)]
    auto_stage_on_change: Option<bool>,
}

impl<'de> Deserialize<'de> for ClientHelloCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ClientHelloCommandWire::deserialize(deserializer)?;
        let staging_host_root = match (wire.staging_host_root, wire.staging_win_root) {
            (Some(host), Some(legacy)) => {
                if host != legacy {
                    return Err(de::Error::custom(
                        "conflicting stagingHostRoot/stagingWinRoot values",
                    ));
                }
                host
            }
            (Some(host), None) => host,
            (None, Some(legacy)) => legacy,
            (None, None) => {
                return Err(de::Error::missing_field("stagingHostRoot"));
            }
        };

        Ok(Self {
            config: wire.config,
            staging_host_root,
            staging_wsl_root: wire.staging_wsl_root,
            auto_stage_on_change: wire.auto_stage_on_change,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetOptionsCommand {
    pub auto_stage_on_change: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchRepoCommand {
    pub repo_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshCommand {
    pub repo_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageFileCommand {
    pub repo_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadTextFileCommand {
    pub repo_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadImageFileCommand {
    pub repo_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleSelection {
    pub include_root: bool,
    pub top_level_dirs: Vec<String>,
    #[serde(default)]
    pub included_subdirs: Vec<String>,
    #[serde(default)]
    pub excluded_subdirs: Vec<String>,
    #[serde(default)]
    pub excluded_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalExcludes {
    #[serde(default)]
    pub dir_names: Vec<String>,
    #[serde(default)]
    pub dir_suffixes: Vec<String>,
    #[serde(default)]
    pub file_names: Vec<String>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildBundleCommand {
    pub repo_id: String,
    pub preset_id: String,
    pub build_id: String,
    pub selection: BundleSelection,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_excludes: Option<GlobalExcludes>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelBundleBuildCommand {
    pub repo_id: String,
    pub preset_id: String,
    pub build_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRepoTopLevelCommand {
    pub repo_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRepoDirectoryCommand {
    pub repo_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListBundlesCommand {
    pub repo_id: String,
    pub preset_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UiCommand {
    #[serde(rename = "clientHello")]
    ClientHello(ClientHelloCommand),
    #[serde(rename = "setOptions")]
    SetOptions(SetOptionsCommand),
    #[serde(rename = "watchRepo")]
    WatchRepo(WatchRepoCommand),
    #[serde(rename = "refresh")]
    Refresh(RefreshCommand),
    #[serde(rename = "stageFile")]
    StageFile(StageFileCommand),
    #[serde(rename = "readTextFile")]
    ReadTextFile(ReadTextFileCommand),
    #[serde(rename = "readImageFile")]
    ReadImageFile(ReadImageFileCommand),
    #[serde(rename = "buildBundle")]
    BuildBundle(BuildBundleCommand),
    #[serde(rename = "cancelBundleBuild")]
    CancelBundleBuild(CancelBundleBuildCommand),
    #[serde(rename = "getRepoTopLevel")]
    GetRepoTopLevel(GetRepoTopLevelCommand),
    #[serde(rename = "listRepoDirectory")]
    ListRepoDirectory(ListRepoDirectoryCommand),
    #[serde(rename = "listBundles")]
    ListBundles(ListBundlesCommand),
    #[serde(rename = "getTrFleetStatus")]
    GetTrFleetStatus(GetTrFleetStatusCommand),
    #[serde(rename = "trFleetAction")]
    TrFleetAction(TrFleetActionCommand),
    #[serde(rename = "sourceControlStatus")]
    SourceControlStatus(SourceControlStatusCommand),
    #[serde(rename = "sourceControlDiff")]
    SourceControlDiff(SourceControlDiffCommand),
    #[serde(rename = "sourceControlImageDiff")]
    SourceControlImageDiff(SourceControlImageDiffCommand),
    #[serde(rename = "sourceControlAction")]
    SourceControlAction(SourceControlActionCommand),
    #[serde(rename = "importFiles")]
    ImportFiles(ImportFilesCommand),
    #[serde(rename = "worktreeAction")]
    WorktreeAction(WorktreeActionCommand),
    /// Drain and stop this agent. Carries no fields and targets no repository:
    /// it is the process-wide shutdown gate, not a repo operation.
    #[serde(rename = "shutdown")]
    Shutdown,
    #[serde(other)]
    Unknown,
}

impl UiCommand {
    pub fn command_type(&self) -> &'static str {
        match self {
            UiCommand::ClientHello(_) => "clientHello",
            UiCommand::SetOptions(_) => "setOptions",
            UiCommand::WatchRepo(_) => "watchRepo",
            UiCommand::Refresh(_) => "refresh",
            UiCommand::StageFile(_) => "stageFile",
            UiCommand::ReadTextFile(_) => "readTextFile",
            UiCommand::ReadImageFile(_) => "readImageFile",
            UiCommand::BuildBundle(_) => "buildBundle",
            UiCommand::CancelBundleBuild(_) => "cancelBundleBuild",
            UiCommand::GetRepoTopLevel(_) => "getRepoTopLevel",
            UiCommand::ListRepoDirectory(_) => "listRepoDirectory",
            UiCommand::ListBundles(_) => "listBundles",
            UiCommand::GetTrFleetStatus(_) => "getTrFleetStatus",
            UiCommand::TrFleetAction(_) => "trFleetAction",
            UiCommand::SourceControlStatus(_) => "sourceControlStatus",
            UiCommand::SourceControlDiff(_) => "sourceControlDiff",
            UiCommand::SourceControlImageDiff(_) => "sourceControlImageDiff",
            UiCommand::SourceControlAction(_) => "sourceControlAction",
            UiCommand::ImportFiles(_) => "importFiles",
            UiCommand::WorktreeAction(_) => "worktreeAction",
            UiCommand::Shutdown => "shutdown",
            UiCommand::Unknown => "unknown",
        }
    }

    /// The repository a command targets, or `None` for global commands. Kept
    /// exhaustive on purpose: a new repo-scoped command that is not listed here
    /// must fail to compile rather than silently become unroutable.
    pub fn repo_id(&self) -> Option<&str> {
        match self {
            UiCommand::WatchRepo(command) => Some(&command.repo_id),
            UiCommand::Refresh(command) => Some(&command.repo_id),
            UiCommand::StageFile(command) => Some(&command.repo_id),
            UiCommand::ReadTextFile(command) => Some(&command.repo_id),
            UiCommand::ReadImageFile(command) => Some(&command.repo_id),
            UiCommand::BuildBundle(command) => Some(&command.repo_id),
            UiCommand::CancelBundleBuild(command) => Some(&command.repo_id),
            UiCommand::GetRepoTopLevel(command) => Some(&command.repo_id),
            UiCommand::ListRepoDirectory(command) => Some(&command.repo_id),
            UiCommand::ListBundles(command) => Some(&command.repo_id),
            UiCommand::SourceControlStatus(command) => Some(&command.repo_id),
            UiCommand::SourceControlDiff(command) => Some(&command.repo_id),
            UiCommand::SourceControlImageDiff(command) => Some(&command.repo_id),
            UiCommand::SourceControlAction(command) => Some(&command.repo_id),
            UiCommand::ImportFiles(command) => Some(&command.repo_id),
            UiCommand::WorktreeAction(command) => Some(&command.repo_id),
            UiCommand::ClientHello(_)
            | UiCommand::SetOptions(_)
            | UiCommand::GetTrFleetStatus(_)
            | UiCommand::TrFleetAction(_)
            | UiCommand::Shutdown
            | UiCommand::Unknown => None,
        }
    }
}
