// Path: src-tauri/src/lib/agent/supervisor/probes.rs
// Description: Async supervisor probe helpers for port, websocket auth, and origin compatibility

use super::AgentSupervisor;
use crate::agent::supervisor::websocket_probe::{
    probe_websocket_auth_blocking, probe_websocket_origin_compatibility_blocking,
};
use crate::commands::agent_probe::probe_port_blocking;

impl AgentSupervisor {
    pub(super) async fn probe_listening(&self, port: u16) -> Result<bool, String> {
        tauri::async_runtime::spawn_blocking(move || probe_port_blocking(port).listening)
            .await
            .map_err(|err| format!("Agent probe task failed: {err}"))
    }

    pub(super) async fn probe_websocket_auth(
        &self,
        port: u16,
        token: &str,
    ) -> Result<bool, String> {
        let token = token.to_string();
        tauri::async_runtime::spawn_blocking(move || probe_websocket_auth_blocking(port, &token))
            .await
            .map_err(|err| format!("Agent websocket auth probe task failed: {err}"))
    }

    pub(super) async fn probe_websocket_origin_compatibility(
        &self,
        port: u16,
        token: &str,
        allowed_origins: &[String],
    ) -> Result<bool, String> {
        let token = token.to_string();
        let allowed_origins = allowed_origins.to_vec();
        tauri::async_runtime::spawn_blocking(move || {
            probe_websocket_origin_compatibility_blocking(port, &token, &allowed_origins)
        })
        .await
        .map_err(|err| format!("Agent websocket origin probe task failed: {err}"))
    }
}
