// Path: crates/im_agent/src/repos/mod.rs
// Description: Repository scanning module exports

mod categorizer;
mod generated_code_extensions;
mod ignore_matcher;
mod image_file_reader;
mod mru_index;
mod recent_files_store;
mod repo_directory_listing;
mod repo_top_level;
mod repo_topology_change;
mod repo_watcher;
mod repo_watcher_events;
mod text_file_reader;
mod watcher_error;

pub use categorizer::Categorizer;
pub use image_file_reader::{read_image_file, ImageFileReadResult};
pub use mru_index::MruIndex;
pub use recent_files_store::RecentFilesStore;
pub use repo_directory_listing::{list_repo_directory, RepoDirectoryListing};
pub use repo_top_level::{get_repo_top_level, is_valid_repo_root, TopLevelResult};
pub use repo_watcher::{RepoWatcher, RepoWatcherConfig};
pub use text_file_reader::{read_text_file, TextFileReadResult};
pub use watcher_error::build_mounted_windows_path_warning_event;
