// Path: src-tauri/src/lib/agent/mod.rs
// Description: Host-agent supervisor module exports (with optional Windows WSL backend)

mod host_process_control;
pub mod install;
mod install_host_binary;
mod install_runtime;
mod process_control;
pub mod supervisor;
pub mod types;
mod wsl_process_control;
mod wsl_process_control_commands;

pub use supervisor::AgentSupervisor;
pub use types::{AgentSupervisorConfig, AgentSupervisorResult, AgentWebSocketAuthState};
