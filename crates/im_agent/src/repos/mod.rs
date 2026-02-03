// Path: crates/im_agent/src/repos/mod.rs
// Description: Repository scanning module exports

mod categorizer;
mod ignore_matcher;
mod mru_index;
mod recent_files_store;
mod repo_top_level;
mod repo_watcher;
mod repo_watcher_events;
mod watcher_error;

pub use categorizer::Categorizer;
pub use mru_index::MruIndex;
pub use recent_files_store::RecentFilesStore;
pub use repo_top_level::{get_repo_top_level, is_valid_repo_root, TopLevelResult};
pub use repo_watcher::{RepoWatcher, RepoWatcherConfig};
