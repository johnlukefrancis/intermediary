// Path: src-tauri/src/lib/crosstalk/feed.rs
// Description: Bounded incremental reader of complete Crosstalk JSONL delivery records

use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

const READ_LIMIT: u64 = 256 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Event {
    pub id: String,
    pub kind: String,
    pub from: String,
    pub to: String,
    pub text: String,
    pub sent: u64,
    #[serde(default)]
    pub cwd: Option<String>,
}

#[derive(Default)]
pub struct Batch {
    pub messages: Vec<Event>,
    pub turns: Vec<Event>,
}

pub struct Feed {
    offset: u64,
    pending: Vec<u8>,
    discard_line: bool,
}

impl Feed {
    pub fn at_end(path: &Path) -> std::io::Result<Self> {
        let offset = match std::fs::metadata(path) {
            Ok(meta) => meta.len(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
            Err(error) => return Err(error),
        };
        Ok(Self {
            offset,
            pending: Vec::new(),
            discard_line: false,
        })
    }

    pub fn read(&mut self, path: &Path) -> std::io::Result<Batch> {
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Batch::default())
            }
            Err(error) => return Err(error),
        };
        let end = file.metadata()?.len();
        if end < self.offset {
            self.offset = 0;
            self.pending.clear();
        }
        if end - self.offset > READ_LIMIT {
            self.offset = end - READ_LIMIT;
            self.pending.clear();
            self.discard_line = true;
            super::report("Crosstalk burst exceeded 256 KiB; older bubble events were skipped");
        }
        file.seek(SeekFrom::Start(self.offset))?;
        let mut bytes = Vec::new();
        file.take(READ_LIMIT).read_to_end(&mut bytes)?;
        self.offset += bytes.len() as u64;
        self.pending.extend(bytes);
        let mut batch = Batch::default();
        let mut consumed = 0;
        for (index, byte) in self.pending.iter().enumerate() {
            if *byte != b'\n' {
                continue;
            }
            let line = &self.pending[consumed..index];
            consumed = index + 1;
            if self.discard_line {
                self.discard_line = false;
                continue;
            }
            match serde_json::from_slice::<Event>(line) {
                Ok(message) if message.kind == "message" && !message.text.trim().is_empty() => {
                    batch.messages.push(message);
                    if batch.messages.len() > 3 {
                        batch.messages.remove(0);
                    }
                }
                Ok(turn)
                    if turn.kind == "turn"
                        && matches!(turn.text.as_str(), "working" | "idle")
                        && turn.cwd.as_ref().is_some_and(|cwd| !cwd.is_empty()) =>
                {
                    batch.turns.push(turn)
                }
                Ok(_) => {}
                Err(_) => super::report("Ignored malformed Crosstalk event"),
            }
        }
        self.pending.drain(..consumed);
        if self.pending.len() > READ_LIMIT as usize {
            self.pending.clear();
            self.discard_line = true;
            super::report("Crosstalk record exceeded 256 KiB; bubble event was skipped");
        }
        Ok(batch)
    }
}

#[cfg(test)]
mod tests {
    use super::Feed;
    use std::io::Write;

    #[test]
    fn observes_only_new_complete_deliveries_and_preserves_multiline_utf8() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, "old history is not parsed").unwrap();
        let mut feed = Feed::at_end(file.path()).unwrap();
        let event = "{\"id\":\"a\",\"kind\":\"message\",\"from\":\"claude@home#1\",\"to\":\"codex@app#2\",\"text\":\"Hello 🌊\\nsecond line\",\"sent\":12}\n";
        let split = event.find('🌊').unwrap() + 1;
        file.write_all(&event.as_bytes()[..split]).unwrap();
        assert!(feed.read(file.path()).unwrap().messages.is_empty());
        file.write_all(&event.as_bytes()[split..]).unwrap();
        let messages = feed.read(file.path()).unwrap().messages;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "Hello 🌊\nsecond line");
        assert!(feed.read(file.path()).unwrap().messages.is_empty());
        file.write_all(event.replace("\"message\"", "\"status\"").as_bytes())
            .unwrap();
        assert!(feed.read(file.path()).unwrap().messages.is_empty());
        writeln!(file, "{{\"id\":\"b\",\"kind\":\"turn\",\"from\":\"claude@repo#1\",\"to\":\"\",\"text\":\"idle\",\"sent\":13,\"cwd\":\"/repo\"}}" ).unwrap();
        let batch = feed.read(file.path()).unwrap();
        assert!(batch.messages.is_empty());
        assert_eq!(batch.turns.len(), 1);
        assert_eq!(batch.turns[0].cwd.as_deref(), Some("/repo"));
    }
}
