// Path: crates/im_agent/src/staging/mod.rs
// Description: Staging module exports

mod path_bridge;
mod stager;

pub use path_bridge::{build_staged_paths, wsl_to_windows, PathBridgeConfig, StagedPaths};
pub use path_bridge::{
    build_staged_paths_for_kind, staging_local_path, windows_to_wsl, StagingRootKind,
};
pub use stager::{stage_file, stage_file_for_kind, validate_relative_path, StageResult};
