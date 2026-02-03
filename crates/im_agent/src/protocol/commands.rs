// Path: crates/im_agent/src/protocol/commands.rs
// Description: UI-to-agent command payloads for the WebSocket protocol

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientHelloCommand {
    // TODO(protocol-precision): replace Value with typed AppConfig once shared schema exists.
    pub config: Value,
    pub staging_wsl_root: String,
    pub staging_win_root: String,
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
            UiCommand::GetRepoTopLevel(_) => "getRepoTopLevel",
            UiCommand::ListBundles(_) => "listBundles",
            UiCommand::Unknown => "unknown",
        }
    }
}
