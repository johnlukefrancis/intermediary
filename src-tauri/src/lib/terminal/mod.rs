// Path: src-tauri/src/lib/terminal/mod.rs
// Description: Integrated terminal backend: ConPTY-backed pwsh sessions owned by the Tauri process (module tree)

mod clipboard;
mod exit_cell;
mod flow_gate;
pub mod frames;
mod output_sink;
mod reader_thread;
mod reaper;
mod registry;
mod registry_shutdown;
#[cfg(test)]
mod registry_tests;
mod session;
mod session_close;
mod session_open;
mod session_spawn;
mod session_spawn_cleanup;
#[cfg(all(test, unix))]
mod session_spawn_tests;
mod shell;
mod start_dir;
mod transaction;
mod waiter_thread;
mod windows_build;
#[cfg(windows)]
mod windows_command_line;
#[cfg(windows)]
mod windows_process;
#[cfg(windows)]
mod windows_pty;
mod worker_start;

pub use clipboard::read_text as read_clipboard_text;
pub use registry::TerminalRegistry;
pub use session_open::open_session;
