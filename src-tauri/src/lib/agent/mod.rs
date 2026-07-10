// Path: src-tauri/src/lib/agent/mod.rs
// Description: Host-agent supervisor module exports (with optional Windows WSL backend)

mod host_process_control;
pub mod install;
mod install_host_binary;
mod install_runtime;
mod process_control;
mod runtime_identity;
pub mod supervisor;
pub mod types;
mod websocket_auth;
mod wsl_command_runner;
mod wsl_process_control;
mod wsl_process_control_commands;
mod wsl_shutdown;

pub use supervisor::AgentSupervisor;
pub use types::{AgentSupervisorConfig, AgentSupervisorResult};
pub use websocket_auth::{AgentWebSocketAuth, AgentWebSocketAuthState};
