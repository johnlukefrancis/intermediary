// Path: crates/im_agent/src/repos/mru_index.rs
// Description: MRU index for recent file changes

use crate::protocol::FileEntry;

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

    pub fn upsert(&mut self, entry: FileEntry) {
        self.entries.retain(|item| item.path != entry.path);
        self.entries.insert(0, entry);
        if self.entries.len() > self.capacity {
            self.entries.truncate(self.capacity);
        }
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
        for entry in persisted {
            if entries
                .iter()
                .any(|item: &FileEntry| item.path == entry.path)
            {
                continue;
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
