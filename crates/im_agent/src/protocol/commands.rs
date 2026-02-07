// Path: crates/im_agent/src/protocol/commands.rs
// Description: UI-to-agent command payloads for the WebSocket protocol

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientHelloCommand {
    // TODO(protocol-precision): replace Value with typed AppConfig once shared schema exists.
    pub config: Value,
    #[serde(alias = "stagingWinRoot")]
    pub staging_host_root: String,
    #[serde(default)]
    pub staging_wsl_root: Option<String>,
    pub auto_stage_on_change: Option<bool>,
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
pub struct BundleSelection {
    pub include_root: bool,
    pub top_level_dirs: Vec<String>,
    #[serde(default)]
    pub excluded_subdirs: Vec<String>,
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
    pub selection: BundleSelection,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_excludes: Option<GlobalExcludes>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRepoTopLevelCommand {
    pub repo_id: String,
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
    #[serde(rename = "buildBundle")]
    BuildBundle(BuildBundleCommand),
    #[serde(rename = "getRepoTopLevel")]
    GetRepoTopLevel(GetRepoTopLevelCommand),
    #[serde(rename = "listBundles")]
    ListBundles(ListBundlesCommand),
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
            UiCommand::BuildBundle(_) => "buildBundle",
            UiCommand::GetRepoTopLevel(_) => "getRepoTopLevel",
            UiCommand::ListBundles(_) => "listBundles",
            UiCommand::Unknown => "unknown",
        }
    }
}
