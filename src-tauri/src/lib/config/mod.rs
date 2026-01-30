// Path: src-tauri/src/lib/config/mod.rs
// Description: Configuration persistence module

pub mod io;
pub mod types;

pub use io::{load_from_disk, save_to_disk, ConfigError};
pub use types::{validate_config, LoadConfigResult, PersistedConfig};
