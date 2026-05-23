// Path: crates/im_agent/src/repos/recent_files_store_tests.rs
// Description: Recent files persistence migration regression tests

use serde_json::json;
use tempfile::tempdir;

use crate::logging::{LogConfig, LogLevel, Logger};
use crate::protocol::FileKind;
use crate::repos::RecentFilesStore;

#[tokio::test]
async fn load_reclassifies_stale_image_entries_for_supported_schemas() {
    for version in [1, 2, 3] {
        let temp = tempdir().expect("temp dir");
        let state_dir = temp.path().join("state");
        let cache_dir = state_dir.join("recent_files");
        let repo_id = format!("repo-{version}");
        let cache_path = cache_dir.join(format!("{repo_id}.json"));

        tokio::fs::create_dir_all(&cache_dir)
            .await
            .expect("state dir");
        tokio::fs::write(&cache_path, stale_image_cache(version, &repo_id))
            .await
            .expect("write cache");

        let logger = Logger::init(LogConfig {
            log_dir: temp.path().join("logs"),
            min_level: LogLevel::Error,
            emit_stdio: false,
        })
        .await
        .expect("logger");
        let store = RecentFilesStore::new(state_dir, logger);

        let entries = store.load(&repo_id, "/repo").await;
        assert_eq!(entries[0].kind, FileKind::Image);
        assert!(entries[0].activity.is_some());

        store.flush_repo(&repo_id).await;
        let saved = read_saved_cache(&cache_path).await;
        assert_eq!(saved["version"], 3);
        assert_eq!(saved["entries"][0]["kind"], "image");
        assert_eq!(saved["entries"][0]["activity"]["history"][0]["count"], 1);
    }
}

fn stale_image_cache(version: u32, repo_id: &str) -> String {
    json!({
        "version": version,
        "repoId": repo_id,
        "repoRoot": "/repo",
        "updatedAtIso": "2026-05-23T12:00:00Z",
        "entries": [{
            "path": "docs/screens/frame.png",
            "kind": "docs",
            "changeType": "change",
            "mtime": "2026-05-23T12:00:00Z",
            "sizeBytes": 10
        }]
    })
    .to_string()
}

async fn read_saved_cache(path: &std::path::Path) -> serde_json::Value {
    let content = tokio::fs::read_to_string(path).await.expect("saved cache");
    serde_json::from_str(&content).expect("saved json")
}
