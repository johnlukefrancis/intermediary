// Path: src-tauri/src/lib/agent/mod.rs
// Description: WSL agent supervisor module exports

pub mod install;
pub mod supervisor;
pub mod types;

pub use supervisor::AgentSupervisor;
pub use types::{AgentSupervisorConfig, AgentSupervisorResult};
