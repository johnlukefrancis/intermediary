// Path: src-tauri/src/lib/config/types/ui_state.rs
// Description: Remembered UI state: rail section, left files mode, rail width, and window bounds

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Remembered UI choices
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiState {
    #[serde(default = "default_main_view")]
    pub main_view: String,
    /// Last active repo (by repoId)
    pub last_active_tab_id: Option<String>,
    /// Last active repo per group (groupId -> repoId)
    #[serde(default)]
    pub last_active_group_repo_ids: HashMap<String, String>,
    /// Remembered window bounds by mode key (standard/handset)
    #[serde(default)]
    pub window_bounds_by_mode: HashMap<String, UiWindowBounds>,
    /// Right-rail section, mirrored by the frontend ActiveRailSchema.
    #[serde(default = "default_active_rail")]
    pub active_rail: String,
    /// Left file panel mode, mirrored by the frontend FilesModeSchema.
    #[serde(default = "default_files_mode")]
    pub files_mode: String,
    /// Rail share of the deck width in standard layout (20-70), set by the drag divider
    #[serde(default = "default_rail_width_percent")]
    pub rail_width_percent: u8,
}

pub const RAIL_WIDTH_PERCENT_MIN: u8 = 20;
pub const RAIL_WIDTH_PERCENT_MAX: u8 = 70;
pub const MAIN_VIEWS: [&str; 2] = ["repository", "overview"];

fn default_main_view() -> String {
    "repository".to_string()
}

fn default_rail_width_percent() -> u8 {
    35
}

/// Rail sections the deck can show; the frontend enum must stay in lockstep
pub const ACTIVE_RAILS: [&str; 3] = ["zips", "source", "terminal"];

fn default_active_rail() -> String {
    "zips".to_string()
}

/// Left file panel modes; the frontend `FILES_MODES` must stay in lockstep
pub const FILES_MODES: [&str; 4] = ["stream", "auto", "latest", "active"];

fn default_files_mode() -> String {
    "stream".to_string()
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            main_view: default_main_view(),
            last_active_tab_id: None,
            last_active_group_repo_ids: HashMap::new(),
            window_bounds_by_mode: HashMap::new(),
            active_rail: default_active_rail(),
            files_mode: default_files_mode(),
            rail_width_percent: default_rail_width_percent(),
        }
    }
}

/// Window bounds persisted for a specific mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiWindowBounds {
    pub width: u32,
    pub height: u32,
}

#[cfg(test)]
mod tests {
    use super::UiState;

    #[test]
    fn overview_round_trip_preserves_the_selected_repository() {
        let state = UiState {
            main_view: "overview".to_string(),
            last_active_tab_id: Some("worktree".to_string()),
            ..UiState::default()
        };
        let value = serde_json::to_value(state).expect("serialize ui state");
        assert_eq!(value["mainView"], "overview");
        let restored: UiState = serde_json::from_value(value).expect("restore ui state");
        assert_eq!(restored.last_active_tab_id.as_deref(), Some("worktree"));
        assert_eq!(restored.main_view, "overview");
        let old: UiState = serde_json::from_str("{\"lastActiveTabId\":null}")
            .expect("load existing config");
        assert_eq!(old.main_view, "repository");
    }
}
