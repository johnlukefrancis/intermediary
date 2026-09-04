// Path: crates/im_agent/src/staging/mod.rs
// Description: Staging module exports

mod layout;
mod layout_unc;
mod stager;

pub use layout::{
    windows_to_wsl, wsl_to_windows, PathBridgeConfig, StagedPaths, StagingLayout, StagingRootKind,
};
pub use layout_unc::unc_to_wsl;
pub use stager::{
    stage_file, stage_file_for_kind, validate_relative_path, StageFileCancelToken, StageResult,
};
pub(crate) use stager::temp_path_for;
