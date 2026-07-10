// Path: src-tauri/src/lib/mod.rs
// Description: Library root - Tauri setup and plugin registration

mod agent;
mod commands;
pub mod config;
pub mod obs;
pub mod paths;

use agent::{AgentSupervisor, AgentWebSocketAuthState};
use commands::agent_control::{ensure_agent_running, restart_agent, stop_agent};
use commands::agent_probe::probe_agent_port;
use commands::config::{load_config, save_config};
use commands::file_manager::open_in_file_manager;
use commands::file_opener::{
    open_file, open_files, reveal_host_file_in_file_manager, reveal_in_file_manager,
};
use commands::notes::{delete_note, load_note, save_note};
use commands::paths::{
    convert_windows_to_wsl, convert_wsl_to_windows, get_app_paths, resolve_repo_root,
};
use commands::reset::reset_app_state;
use commands::startup::{apply_launch_window_bounds, startup_ready, StartupWindowState};
use commands::wsl_distro::WslDistroState;
use obs::logging;
use tauri::{Manager, RunEvent};

/// Run the Tauri application
pub fn run() {
    let context = tauri::generate_context!();
    logging::init_before_tauri(&context.config().identifier);
    logging::install_panic_hook();
    logging::log(
        "info",
        "app",
        "startup_begin",
        &format!(
            "Process entered Rust startup before Tauri construction pid={}",
            std::process::id()
        ),
    );

    let app_local_data = match dirs::data_local_dir() {
        Some(base) => base.join(&context.config().identifier),
        None => {
            logging::log(
                "error",
                "app",
                "startup_state_failed",
                "Failed to resolve app local data directory before WebView creation",
            );
            return;
        }
    };
    let auth_state = match AgentWebSocketAuthState::from_app_local_data(&app_local_data) {
        Ok(state) => state,
        Err(err) => {
            logging::log("error", "app", "startup_state_failed", &err);
            return;
        }
    };

    let app = tauri::Builder::default()
        .manage(StartupWindowState::default())
        .manage(AgentSupervisor::new())
        .manage(WslDistroState::default())
        .manage(auth_state)
        .plugin(tauri_plugin_drag::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|_| {
            logging::log("info", "app", "setup_entered", "Tauri setup entered");
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
            reveal_host_file_in_file_manager,
            open_file,
            open_files,
            startup_ready,
            load_note,
            save_note,
            delete_note
        ])
        .build(context);

    let app = match app {
        Ok(app) => app,
        Err(err) => {
            let message = format!("Failed to build Tauri application: {err}");
            logging::log("error", "app", "build_failed", &message);
            eprintln!("{message}");
            return;
        }
    };
    logging::log(
        "info",
        "app",
        "build_complete",
        "Tauri application construction completed",
    );

    let mut stopped = false;
    app.run(move |app_handle, event| {
        if matches!(event, RunEvent::Ready) {
            apply_launch_window_bounds(app_handle);
        }

        if !stopped && matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit) {
            if let Some(supervisor) = app_handle.try_state::<AgentSupervisor>() {
                if let Err(err) = tauri::async_runtime::block_on(supervisor.shutdown_on_exit()) {
                    logging::log("error", "agent", "stop_on_exit_failed", &err);
                }
            } else {
                logging::log(
                    "error",
                    "agent",
                    "stop_on_exit_state_missing",
                    "Agent supervisor was unavailable during application exit",
                );
            }
            stopped = true;
        }
    });
}
