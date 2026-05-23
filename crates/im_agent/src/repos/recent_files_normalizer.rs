// Path: crates/im_agent/src/repos/recent_files_normalizer.rs
// Description: Normalize persisted recent-file entries against current filters

use crate::protocol::{FileEntry, FileKind};
use crate::repos::categorizer::{is_image_path, Categorizer};
use crate::repos::ignore_matcher::IgnoreMatcher;
use crate::repos::{activity_from_mtime, normalize_activity_history};

pub(crate) struct RecentFilesNormalization {
    pub(crate) entries: Vec<FileEntry>,
    pub(crate) needs_save: bool,
}

pub(crate) fn normalize_persisted_entries(
    entries: Vec<FileEntry>,
    schema_needs_save: bool,
    categorizer: Option<&Categorizer>,
    ignore_matcher: Option<&IgnoreMatcher>,
) -> RecentFilesNormalization {
    let mut normalized_entries = Vec::with_capacity(entries.len());
    let mut needs_save = schema_needs_save;

    for mut entry in entries {
        needs_save = normalize_path(&mut entry) || needs_save;

        if is_ignored(&entry, ignore_matcher) {
            needs_save = true;
            continue;
        }

        match normalize_entry_kind(&mut entry, categorizer) {
            EntryKindNormalization::Changed => needs_save = true,
            EntryKindNormalization::Dropped => {
                needs_save = true;
                continue;
            }
            EntryKindNormalization::Unchanged => {}
        }

        needs_save = normalize_entry_activity(&mut entry) || needs_save;
        normalized_entries.push(entry);
    }

    RecentFilesNormalization {
        entries: normalized_entries,
        needs_save,
    }
}

enum EntryKindNormalization {
    Unchanged,
    Changed,
    Dropped,
}

fn normalize_path(entry: &mut FileEntry) -> bool {
    let normalized_path = entry.path.replace('\\', "/");
    if normalized_path == entry.path {
        return false;
    }

    entry.path = normalized_path;
    true
}

fn is_ignored(entry: &FileEntry, ignore_matcher: Option<&IgnoreMatcher>) -> bool {
    ignore_matcher
        .map(|matcher| matcher.should_ignore(&entry.path))
        .unwrap_or(false)
}

fn normalize_entry_kind(
    entry: &mut FileEntry,
    categorizer: Option<&Categorizer>,
) -> EntryKindNormalization {
    let Some(categorizer) = categorizer else {
        if is_image_path(&entry.path) && entry.kind != FileKind::Image {
            entry.kind = FileKind::Image;
            return EntryKindNormalization::Changed;
        }
        return EntryKindNormalization::Unchanged;
    };

    let current_kind = categorizer.categorize(&entry.path);
    if current_kind == FileKind::Other {
        return EntryKindNormalization::Dropped;
    }
    if entry.kind == current_kind {
        return EntryKindNormalization::Unchanged;
    }

    entry.kind = current_kind;
    EntryKindNormalization::Changed
}

fn normalize_entry_activity(entry: &mut FileEntry) -> bool {
    let mut changed = false;
    if entry.activity.is_none() {
        entry.activity = Some(activity_from_mtime(&entry.mtime));
        changed = true;
    }

    if let Some(activity) = &mut entry.activity {
        changed = normalize_activity_history(activity, &entry.mtime) || changed;
    }
    changed
}
