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
#[serde(tag = "type")]
pub enum UiCommand {
    #[serde(rename = "clientHello")]
    ClientHello(ClientHelloCommand),
    #[serde(rename = "setOptions")]
    SetOptions(SetOptionsCommand),
    #[serde(other)]
    Unknown,
}

impl UiCommand {
    pub fn command_type(&self) -> &'static str {
        match self {
            UiCommand::ClientHello(_) => "clientHello",
            UiCommand::SetOptions(_) => "setOptions",
            UiCommand::Unknown => "unknown",
        }
    }
}
