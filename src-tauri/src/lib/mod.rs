// Path: src-tauri/src/lib/mod.rs
// Description: Library root - Tauri setup and plugin registration

mod agent;
mod commands;
pub mod config;
pub mod obs;
pub mod paths;
mod terminal;
mod wsl_control;

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
use commands::startup::{
    apply_launch_window_bounds, retire_splashscreen, startup_ready, StartupWindowState,
};
use commands::terminal::{
    terminal_ack, terminal_clipboard_text, terminal_close, terminal_open, terminal_resize,
    terminal_write,
};
use commands::wsl_distro::WslDistroState;
use obs::logging;
use tauri::webview::PageLoadEvent;
use tauri::{Manager, RunEvent};
use terminal::frames::CloseReason;
use terminal::TerminalRegistry;

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
        .manage(TerminalRegistry::default())
        .plugin(tauri_plugin_drag::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|_| {
            logging::log("info", "app", "setup_entered", "Tauri setup entered");
            Ok(())
        })
        // A navigation or reload of the main page orphans every terminal channel:
        // the sessions opened for the old page are closed (I1), detached from
        // this main-thread hook.
        .on_page_load(|webview, payload| {
            if webview.label() != "main" || payload.event() != PageLoadEvent::Started {
                return;
            }
            if let Some(registry) = webview.try_state::<TerminalRegistry>() {
                registry.close_all_detached(CloseReason::WebviewNavigation);
            }
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
            delete_note,
            terminal_open,
            terminal_write,
            terminal_resize,
            terminal_ack,
            terminal_close,
            terminal_clipboard_text
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

        if let RunEvent::WindowEvent { label, event, .. } = &event {
            if label == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                retire_splashscreen(app_handle);
            }
        }

        if !stopped && matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit) {
            // Terminals first (I4): their wsl.exe children must be gone before the
            // supervisor's WSL idle probe decides whether the distro can be freed.
            let terminals_settled =
                if let Some(registry) = app_handle.try_state::<TerminalRegistry>() {
                    match registry.shutdown_all_blocking() {
                        Ok(()) => true,
                        Err(err) => {
                            logging::log("error", "terminal", "shutdown_incomplete", &err);
                            false
                        }
                    }
                } else {
                    logging::log(
                        "error",
                        "terminal",
                        "shutdown_all",
                        "Terminal registry was unavailable during application exit",
                    );
                    false
                };
            if terminals_settled {
                if let Some(supervisor) = app_handle.try_state::<AgentSupervisor>() {
                    if let Err(err) = tauri::async_runtime::block_on(supervisor.shutdown_on_exit())
                    {
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
            } else {
                logging::log(
                    "error",
                    "agent",
                    "stop_on_exit_skipped",
                    "Agent exit teardown was skipped because terminal finality was not proved",
                );
            }
            stopped = true;
        }
    });
}
