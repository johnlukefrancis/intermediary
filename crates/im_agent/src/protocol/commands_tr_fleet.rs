// Path: crates/im_agent/src/protocol/commands_tr_fleet.rs
// Description: TR fleet command payloads for host-agent build-server status and recovery controls

use serde::{Deserialize, Serialize};

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
