// Path: src-tauri/src/lib/agent/mod.rs
// Description: Host-agent supervisor module exports (with optional Windows WSL backend)

pub mod install;
mod process_control;
pub mod supervisor;
mod supervisor_helpers;
pub mod types;
mod wsl_process_control;
mod wsl_process_control_commands;

pub use supervisor::AgentSupervisor;
pub use types::{AgentSupervisorConfig, AgentSupervisorResult, AgentWebSocketAuthState};
