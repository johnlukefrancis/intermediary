// Path: crates/im_agent/src/runtime/state.rs
// Description: Agent runtime state and option handlers

use serde_json::Value;

use crate::protocol::{
    ClientHelloCommand, ClientHelloResult, SetOptionsCommand, SetOptionsResult,
};

#[derive(Debug)]
pub struct AgentRuntime {
    pub config: Option<Value>,
    pub staging_wsl_root: Option<String>,
    pub staging_win_root: Option<String>,
    pub auto_stage_on_change: bool,
}

impl AgentRuntime {
    pub fn new() -> Self {
        Self {
            config: None,
            staging_wsl_root: None,
            staging_win_root: None,
            auto_stage_on_change: true,
        }
    }

    pub fn apply_client_hello(
        &mut self,
        command: ClientHelloCommand,
        agent_version: &str,
    ) -> ClientHelloResult {
        let resolved_auto_stage = resolve_auto_stage(&command, self.auto_stage_on_change);

        self.config = Some(command.config);
        self.staging_wsl_root = Some(command.staging_wsl_root);
        self.staging_win_root = Some(command.staging_win_root);
        self.auto_stage_on_change = resolved_auto_stage;

        ClientHelloResult {
            agent_version: agent_version.to_string(),
            watched_repo_ids: Vec::new(),
        }
    }

    pub fn apply_set_options(&mut self, command: SetOptionsCommand) -> SetOptionsResult {
        if let Some(value) = command.auto_stage_on_change {
            self.auto_stage_on_change = value;
        }

        SetOptionsResult {
            auto_stage_on_change: self.auto_stage_on_change,
        }
    }
}

fn resolve_auto_stage(command: &ClientHelloCommand, fallback: bool) -> bool {
    if let Some(value) = command.auto_stage_on_change {
        return value;
    }

    command
        .config
        .get("autoStageGlobal")
        .and_then(|value| value.as_bool())
        .unwrap_or(fallback)
}
