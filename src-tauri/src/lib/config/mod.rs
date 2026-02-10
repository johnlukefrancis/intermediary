// Path: src-tauri/src/lib/config/mod.rs
// Description: Configuration persistence module

pub(crate) mod generated_code_globs;
pub mod io;
pub mod path;
pub mod types;

pub use io::{load_from_disk, save_to_disk, ConfigError};
pub use path::resolve_config_path;
pub use types::{validate_config, LoadConfigResult, PersistedConfig};
