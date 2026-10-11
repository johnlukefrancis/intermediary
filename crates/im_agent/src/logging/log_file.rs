// Path: crates/im_agent/src/logging/log_file.rs
// Description: One agent's JSONL log file, rotated by size into a single previous generation

use std::io;
use std::path::{Path, PathBuf};

use tokio::fs::{self, File, OpenOptions};
use tokio::io::AsyncWriteExt;

/// Rotation threshold per file; with one previous generation an agent keeps at most twice this.
pub const LOG_ROTATE_BYTES: u64 = 16 * 1024 * 1024;

pub(super) struct LogFile {
    latest: PathBuf,
    previous: PathBuf,
    file: Option<File>,
    written: u64,
    limit: u64,
}

/// `<stem>_latest.log` and `<stem>_previous.log` inside `log_dir`.
pub fn log_file_paths(log_dir: &Path, stem: &str) -> (PathBuf, PathBuf) {
    (
        log_dir.join(format!("{stem}_latest.log")),
        log_dir.join(format!("{stem}_previous.log")),
    )
}

impl LogFile {
    pub(super) async fn open(log_dir: &Path, stem: &str, limit: u64) -> io::Result<Self> {
        let (latest, previous) = log_file_paths(log_dir, stem);
        let mut log = Self {
            file: Some(open_append(&latest).await?),
            written: fs::metadata(&latest).await.map(|meta| meta.len())?,
            latest,
            previous,
            limit,
        };
        if log.written >= limit {
            log.rotate().await?;
        }
        Ok(log)
    }

    pub(super) async fn append_line(&mut self, line: &str) -> io::Result<()> {
        let len = line.len() as u64 + 1;
        if self.written > 0 && self.written.saturating_add(len) > self.limit {
            self.rotate().await?;
        }
        let file = match self.file.as_mut() {
            Some(file) => file,
            None => self.file.insert(open_append(&self.latest).await?),
        };
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;
        self.written = self.written.saturating_add(len);
        Ok(())
    }

    /// Closes before renaming (Windows refuses to rename an open file); a failed rename
    /// truncates instead, so the size bound holds even when a reader pins the old name.
    async fn rotate(&mut self) -> io::Result<()> {
        if let Some(mut file) = self.file.take() {
            file.flush().await?;
        }
        if fs::rename(&self.latest, &self.previous).await.is_err() {
            let file = OpenOptions::new().write(true).open(&self.latest).await?;
            file.set_len(0).await?;
        }
        self.file = Some(open_append(&self.latest).await?);
        self.written = 0;
        Ok(())
    }
}

async fn open_append(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await
}

#[cfg(test)]
mod tests {
    use super::{log_file_paths, LogFile};

    #[tokio::test]
    async fn crossing_the_limit_keeps_one_previous_generation() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (latest, previous) = log_file_paths(dir.path(), "agent_test");
        let mut log = LogFile::open(dir.path(), "agent_test", 32)
            .await
            .expect("open");
        for line in [
            "first line of twenty",
            "second line crosses",
            "third line rotates",
        ] {
            log.append_line(line).await.expect("append");
        }
        let latest_text = std::fs::read_to_string(&latest).expect("latest");
        let previous_text = std::fs::read_to_string(&previous).expect("previous");
        assert_eq!(latest_text, "third line rotates\n");
        assert_eq!(previous_text, "second line crosses\n");
    }

    #[tokio::test]
    async fn an_oversized_file_from_a_previous_run_rotates_at_open() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (latest, previous) = log_file_paths(dir.path(), "agent_test");
        std::fs::write(&latest, vec![b'x'; 64]).expect("seed");
        let mut log = LogFile::open(dir.path(), "agent_test", 32)
            .await
            .expect("open");
        log.append_line("fresh").await.expect("append");
        assert_eq!(std::fs::read_to_string(&latest).expect("latest"), "fresh\n");
        assert_eq!(std::fs::metadata(&previous).expect("previous").len(), 64);
    }
}
