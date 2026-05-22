// Path: crates/im_agent/src/repos/repo_topology_change.rs
// Description: Detect watcher events that invalidate repo top-level metadata

use notify::event::{CreateKind, ModifyKind, RemoveKind};
use notify::{Event, EventKind};
use std::path::{Path, PathBuf};

const ROOT_FILE_METADATA_DEPTH: usize = 1;
const DIRECTORY_SELECTOR_METADATA_DEPTH: usize = 4;

pub(crate) async fn event_affects_top_level_metadata(root_path: &Path, event: &Event) -> bool {
    match &event.kind {
        EventKind::Create(CreateKind::Folder) | EventKind::Remove(RemoveKind::Folder) => {
            any_path_at_or_above_depth(root_path, &event.paths, DIRECTORY_SELECTOR_METADATA_DEPTH)
        }
        EventKind::Create(CreateKind::File) | EventKind::Remove(RemoveKind::File) => {
            any_path_at_or_above_depth(root_path, &event.paths, ROOT_FILE_METADATA_DEPTH)
        }
        EventKind::Create(CreateKind::Any) => {
            any_path_at_or_above_depth(root_path, &event.paths, ROOT_FILE_METADATA_DEPTH)
                || any_existing_dir_at_or_above_depth(
                    root_path,
                    &event.paths,
                    DIRECTORY_SELECTOR_METADATA_DEPTH,
                )
                .await
        }
        EventKind::Remove(RemoveKind::Any) | EventKind::Modify(ModifyKind::Name(_)) => {
            any_path_at_or_above_depth(root_path, &event.paths, DIRECTORY_SELECTOR_METADATA_DEPTH)
        }
        _ => false,
    }
}

fn any_path_at_or_above_depth(root_path: &Path, paths: &[PathBuf], max_depth: usize) -> bool {
    paths
        .iter()
        .any(|path| relative_depth(root_path, path).is_some_and(|depth| depth <= max_depth))
}

async fn any_existing_dir_at_or_above_depth(
    root_path: &Path,
    paths: &[PathBuf],
    max_depth: usize,
) -> bool {
    for path in paths {
        let Some(depth) = relative_depth(root_path, path) else {
            continue;
        };
        if depth > max_depth {
            continue;
        }
        if tokio::fs::metadata(path)
            .await
            .map(|metadata| metadata.is_dir())
            .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

fn relative_depth(root_path: &Path, path: &Path) -> Option<usize> {
    let relative = path.strip_prefix(root_path).ok()?;
    let depth = relative
        .components()
        .filter(|component| matches!(component, std::path::Component::Normal(_)))
        .count();
    (depth > 0).then_some(depth)
}

#[cfg(test)]
mod tests {
    use super::event_affects_top_level_metadata;
    use notify::event::{CreateKind, ModifyKind, RenameMode};
    use notify::{Event, EventKind};
    use std::path::Path;

    #[tokio::test]
    async fn folder_create_under_top_level_invalidates_metadata() {
        let event = Event::new(EventKind::Create(CreateKind::Folder))
            .add_path("/repo/Docs/Screenshots".into());

        assert!(event_affects_top_level_metadata(Path::new("/repo"), &event).await);
    }

    #[tokio::test]
    async fn nested_file_create_does_not_invalidate_top_level_metadata() {
        let event = Event::new(EventKind::Create(CreateKind::File))
            .add_path("/repo/Docs/Screenshots/a.png".into());

        assert!(!event_affects_top_level_metadata(Path::new("/repo"), &event).await);
    }

    #[tokio::test]
    async fn depth_two_rename_invalidates_potential_subdir_metadata() {
        let event = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path("/repo/Docs/Old".into())
            .add_path("/repo/Docs/New".into());

        assert!(event_affects_top_level_metadata(Path::new("/repo"), &event).await);
    }

    #[tokio::test]
    async fn depth_four_folder_create_invalidates_selector_metadata() {
        let event = Event::new(EventKind::Create(CreateKind::Folder))
            .add_path("/repo/Docs/Architecture/ADRs/Drafts".into());

        assert!(event_affects_top_level_metadata(Path::new("/repo"), &event).await);
    }

    #[tokio::test]
    async fn depth_five_folder_create_does_not_invalidate_selector_metadata() {
        let event = Event::new(EventKind::Create(CreateKind::Folder))
            .add_path("/repo/Docs/Architecture/ADRs/Drafts/Archive".into());

        assert!(!event_affects_top_level_metadata(Path::new("/repo"), &event).await);
    }
}
