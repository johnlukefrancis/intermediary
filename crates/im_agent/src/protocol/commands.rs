// Path: crates/im_agent/src/protocol/commands.rs
// Description: UI-to-agent command payloads for the WebSocket protocol

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTrFleetStatusCommand {}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrFleetWatchBackend {
    #[default]
    Auto,
    Native,
    Poll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum TrFleetActionPayload {
    Rebuild {
        port: u16,
    },
    RestartWatch {
        port: u16,
        #[serde(default)]
        backend: TrFleetWatchBackend,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrFleetActionCommand {
    #[serde(flatten)]
    pub payload: TrFleetActionPayload,
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
    #[serde(rename = "getTrFleetStatus")]
    GetTrFleetStatus(GetTrFleetStatusCommand),
    #[serde(rename = "trFleetAction")]
    TrFleetAction(TrFleetActionCommand),
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
            UiCommand::GetTrFleetStatus(_) => "getTrFleetStatus",
            UiCommand::TrFleetAction(_) => "trFleetAction",
            UiCommand::Unknown => "unknown",
        }
    }
}
