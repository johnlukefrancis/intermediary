// Path: crates/im_agent/src/repos/mru_index.rs
// Description: MRU index for recent file changes

use chrono::{DateTime, Utc};

use crate::protocol::FileEntry;
use crate::repos::{activity_from_mtime, normalize_activity_history, update_activity};

#[derive(Debug)]
pub struct MruIndex {
    capacity: usize,
    entries: Vec<FileEntry>,
}

impl MruIndex {
    pub fn new(capacity: usize) -> Result<Self, String> {
        if capacity == 0 {
            return Err("MruIndex capacity must be positive".to_string());
        }
        Ok(Self {
            capacity,
            entries: Vec::new(),
        })
    }

    pub fn upsert(&mut self, entry: FileEntry) -> FileEntry {
        self.upsert_at(entry, Utc::now())
    }

    pub(crate) fn upsert_at(
        &mut self,
        mut entry: FileEntry,
        observed_at: DateTime<Utc>,
    ) -> FileEntry {
        let previous_activity = self
            .entries
            .iter()
            .find(|item| item.path == entry.path)
            .and_then(|item| item.activity.as_ref());
        entry.activity = Some(update_activity(previous_activity, observed_at));

        self.entries.retain(|item| item.path != entry.path);
        self.entries.insert(0, entry);
        if self.entries.len() > self.capacity {
            self.entries.truncate(self.capacity);
        }
        self.entries[0].clone()
    }

    pub fn remove(&mut self, path: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|item| item.path != path);
        before != self.entries.len()
    }

    pub fn entries(&self) -> Vec<FileEntry> {
        self.entries.clone()
    }

    pub fn load_from(&mut self, persisted: Vec<FileEntry>) {
        let mut entries = Vec::new();
        for mut entry in persisted {
            if entries
                .iter()
                .any(|item: &FileEntry| item.path == entry.path)
            {
                continue;
            }
            if entry.activity.is_none() {
                entry.activity = Some(activity_from_mtime(&entry.mtime));
            }
            if let Some(activity) = &mut entry.activity {
                normalize_activity_history(activity, &entry.mtime);
            }
            entries.push(entry);
            if entries.len() == self.capacity {
                break;
            }
        }
        self.entries = entries;
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn size(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{FileChangeType, FileKind};

    #[test]
    fn timestamp_preserving_edits_update_activity_at_observation_time() {
        let mut index = MruIndex::new(8).expect("index");
        let entry = FileEntry {
            path: "src/main.rs".into(),
            kind: FileKind::Code,
            change_type: FileChangeType::Change,
            mtime: "2001-01-01T00:00:00Z".into(),
            size_bytes: None,
            activity: None,
        };
        let first = DateTime::parse_from_rfc3339("2026-10-09T12:00:00Z")
            .expect("first time")
            .with_timezone(&Utc);
        let later = first + chrono::Duration::minutes(5);
        index.upsert_at(entry.clone(), first);
        let updated = index.upsert_at(entry, later);
        assert_eq!(updated.mtime, "2001-01-01T00:00:00Z");
        let activity = updated.activity.expect("activity");
        assert_eq!(activity.first_seen_at_iso, first.to_rfc3339());
        assert_eq!(activity.last_seen_at_iso, later.to_rfc3339());
        assert_eq!(activity.update_count, 2);
        assert_eq!(activity.burst_count, 2);
        assert_eq!(activity.history.len(), 1);
        assert_eq!(activity.history[0].bucket_start_iso, first.to_rfc3339());
        assert_eq!(activity.history[0].count, 2);
        assert_eq!(index.size(), 1);
    }
}
