// Path: crates/im_agent/src/runtime/state.rs
// Description: Agent runtime state and option handlers

use std::collections::HashMap;

use serde_json::Value;

use crate::protocol::{
    ClientHelloCommand, ClientHelloResult, SetOptionsCommand, SetOptionsResult,
};
use crate::staging::PathBridgeConfig;

#[derive(Debug)]
pub struct AgentRuntime {
    pub config: Option<Value>,
    pub staging: Option<PathBridgeConfig>,
    pub repo_roots: HashMap<String, String>,
    pub auto_stage_on_change: bool,
}

impl AgentRuntime {
    pub fn new() -> Self {
        Self {
            config: None,
            staging: None,
            repo_roots: HashMap::new(),
            auto_stage_on_change: true,
        }
    }

    pub fn apply_client_hello(
        &mut self,
        command: ClientHelloCommand,
        agent_version: &str,
    ) -> ClientHelloResult {
        let resolved_auto_stage = resolve_auto_stage(&command, self.auto_stage_on_change);

        self.repo_roots = extract_repo_roots(&command.config);
        self.config = Some(command.config);
        self.staging = Some(PathBridgeConfig {
            staging_wsl_root: command.staging_wsl_root,
            staging_win_root: command.staging_win_root,
        });
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

fn extract_repo_roots(config: &Value) -> HashMap<String, String> {
    let mut roots = HashMap::new();
    let Some(repos) = config.get("repos").and_then(|value| value.as_array()) else {
        return roots;
    };

    for repo in repos {
        let Some(repo_id) = repo.get("repoId").and_then(|value| value.as_str()) else {
            continue;
        };
        let Some(wsl_path) = repo.get("wslPath").and_then(|value| value.as_str()) else {
            continue;
        };
        roots.insert(repo_id.to_string(), wsl_path.to_string());
    }

    roots
}
