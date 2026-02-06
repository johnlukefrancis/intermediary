// Path: src-tauri/src/lib/agent/mod.rs
// Description: Host + WSL agent supervisor module exports

pub mod install;
mod process_control;
pub mod supervisor;
mod supervisor_helpers;
pub mod types;

pub use supervisor::AgentSupervisor;
pub use types::{AgentSupervisorConfig, AgentSupervisorResult};
