// Path: src-tauri/src/lib/mod.rs
// Description: Library root - Tauri setup and plugin registration

mod agent;
mod commands;
pub mod config;
pub mod obs;
pub mod paths;

use agent::AgentSupervisor;
use commands::agent_control::{ensure_agent_running, restart_agent, stop_agent};
use commands::agent_probe::probe_agent_port;
use commands::config::{load_config, save_config};
use commands::file_manager::open_in_file_manager;
use commands::file_opener::{open_file, open_files, reveal_in_file_manager};
use commands::paths::{
    convert_windows_to_wsl, convert_wsl_to_windows, get_app_paths, resolve_repo_root,
};
use commands::reset::reset_app_state;
use obs::logging;
use tauri::{Manager, RunEvent};

/// Run the Tauri application
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_drag::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Initialize logging
            if let Some(log_dir) = logging::resolve_log_dir(app.handle()) {
                logging::init(&log_dir);
                logging::log("info", "app", "startup", "Intermediary initialized");
            }
            app.manage(AgentSupervisor::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_paths,
            load_config,
            save_config,
            probe_agent_port,
            ensure_agent_running,
            restart_agent,
            stop_agent,
            reset_app_state,
            convert_windows_to_wsl,
            resolve_repo_root,
            convert_wsl_to_windows,
            open_in_file_manager,
            reveal_in_file_manager,
            open_file,
            open_files
        ])
        .build(tauri::generate_context!())
        .expect("error building tauri application");

    let mut stopped = false;
    app.run(move |app_handle, event| {
        if stopped {
            return;
        }
        if matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit) {
            let supervisor = app_handle.state::<AgentSupervisor>();
            if let Err(err) = tauri::async_runtime::block_on(supervisor.stop()) {
                logging::log("error", "agent", "stop_on_exit_failed", &err);
            }
            stopped = true;
        }
    });
}
