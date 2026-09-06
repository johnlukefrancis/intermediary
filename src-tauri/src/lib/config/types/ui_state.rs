// Path: src-tauri/src/lib/config/types/ui_state.rs
// Description: Remembered UI state: rail section, left files mode, rail width, and window bounds

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Remembered UI choices
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiState {
    /// Last active repo (by repoId)
    pub last_active_tab_id: Option<String>,
    /// Last active repo per group (groupId -> repoId)
    #[serde(default)]
    pub last_active_group_repo_ids: HashMap<String, String>,
    /// Remembered window bounds by mode key (standard/handset)
    #[serde(default)]
    pub window_bounds_by_mode: HashMap<String, UiWindowBounds>,
    /// Right-rail section shown in the deck: `zips`, `source`, or `terminal`.
    /// Mirrors the frontend `ActiveRailSchema`; without this field a save
    /// round-trip dropped the choice and the rail always reloaded on ZIPS.
    #[serde(default = "default_active_rail")]
    pub active_rail: String,
    /// Left file panel mode: `stream`, `auto`, `latest`, or `active`.
    /// Mirrors the frontend `FilesModeSchema`; defaulted, so an older config
    /// loads on STREAM without a schema migration.
    #[serde(default = "default_files_mode")]
    pub files_mode: String,
    /// Rail share of the deck width in standard layout (20-70), set by the drag divider
    #[serde(default = "default_rail_width_percent")]
    pub rail_width_percent: u8,
}

pub const RAIL_WIDTH_PERCENT_MIN: u8 = 20;
pub const RAIL_WIDTH_PERCENT_MAX: u8 = 70;

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
