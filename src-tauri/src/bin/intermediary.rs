// Path: src-tauri/src/bin/intermediary.rs
// Description: Binary entry point for Tauri app

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    intermediary_lib::run();
}
