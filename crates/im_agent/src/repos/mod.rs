// Path: crates/im_agent/src/repos/mod.rs
// Description: Repository scanning module exports

mod categorizer;
mod file_activity;
mod generated_code_extensions;
mod ignore_matcher;
mod image_file_reader;
mod import;
mod mru_index;
mod recent_files_normalizer;
mod recent_files_store;
#[cfg(test)]
mod recent_files_store_tests;
mod repo_directory_listing;
mod repo_top_level;
mod repo_topology_change;
mod repo_watcher;
mod repo_watcher_events;
#[cfg(test)]
mod repo_watcher_tests;
mod source_control_watch;
mod text_file_reader;
mod watcher_error;
pub mod worktree;

pub use categorizer::Categorizer;
pub(crate) use file_activity::{
    activity_from_mtime, normalize_activity_history, observed_at_from_mtime, update_activity,
};
pub use image_file_reader::{read_image_file, ImageFileReadResult};
pub use import::{import_files, MAX_IMPORT_ENTRIES};
pub(crate) use image_file_reader::mime_type_for_path;
pub use mru_index::MruIndex;
pub use recent_files_store::RecentFilesStore;
pub use repo_directory_listing::{
    list_repo_directory, normalize_directory_path, RepoDirectoryListing,
};
pub use repo_top_level::{get_repo_top_level, is_valid_repo_root, TopLevelResult};
pub use repo_watcher::{RepoWatcher, RepoWatcherConfig};
pub use text_file_reader::{read_text_file, TextFileReadResult};
pub use watcher_error::build_mounted_windows_path_warning_event;
