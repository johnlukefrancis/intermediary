// Path: src-tauri/src/lib/mod.rs
// Description: Library root - Tauri setup and plugin registration

mod commands;
pub mod config;
pub mod obs;
pub mod paths;

use commands::config::{load_config, save_config};
use commands::file_manager::open_in_file_manager;
use commands::paths::{convert_windows_to_wsl, convert_wsl_to_windows, get_app_paths};
use commands::wsl::resolve_wsl_host;
use obs::logging;

/// Run the Tauri application
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_drag::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Initialize logging
            if let Some(log_dir) = logging::resolve_log_dir(app.handle()) {
                logging::init(&log_dir);
                logging::log("info", "app", "startup", "Intermediary initialized");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_paths,
            load_config,
            save_config,
            resolve_wsl_host,
            convert_windows_to_wsl,
            convert_wsl_to_windows,
            open_in_file_manager
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
