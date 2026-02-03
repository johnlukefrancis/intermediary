// Path: crates/im_agent/src/staging/mod.rs
// Description: Staging module exports

mod path_bridge;
mod stager;

pub use path_bridge::{build_staged_paths, wsl_to_windows, PathBridgeConfig, StagedPaths};
pub use stager::{stage_file, validate_relative_path, StageResult};
