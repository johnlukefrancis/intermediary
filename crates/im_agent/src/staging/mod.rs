// Path: crates/im_agent/src/staging/mod.rs
// Description: Staging module exports

mod layout;
mod stager;

pub use layout::{
    windows_to_wsl, wsl_to_windows, PathBridgeConfig, StagedPaths, StagingLayout, StagingRootKind,
};
pub use stager::{stage_file, stage_file_for_kind, validate_relative_path, StageResult};
