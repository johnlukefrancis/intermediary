// Path: crates/im_bundle/src/writer_tests.rs
// Description: Tests for bundle writer behavior and progress ordering

use std::io::Read;
use std::sync::{Arc, Mutex};

use tempfile::tempdir;

use crate::plan::{BundleGitInfo, BundleSelection, GlobalExcludes};
use crate::progress::ProgressMessage;
use crate::progress_sink::CallbackProgressSink;
use crate::writer::{write_bundle, write_bundle_with_progress};
use crate::BundlePlan;

#[test]
fn writes_zip_with_manifest_and_respects_exclusions() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    std::fs::create_dir_all(repo_root.join("app/src")).unwrap();
    std::fs::create_dir_all(repo_root.join("dist")).unwrap();
    std::fs::write(repo_root.join("README.md"), "root").unwrap();
    std::fs::write(repo_root.join("app/src/main.ts"), "code").unwrap();
    std::fs::write(repo_root.join("dist/bundle.js"), "ignored").unwrap();

    let output_path = repo_root.join("bundle.zip");
    let plan = BundlePlan {
        output_path: output_path.clone(),
        repo_root: repo_root.to_path_buf(),
        repo_id: "repo".to_string(),
        preset_id: "full".to_string(),
        preset_name: "Full".to_string(),
        selection: BundleSelection {
            include_root: true,
            top_level_dirs: vec!["app".to_string()],
            excluded_subdirs: vec![],
        },
        git: BundleGitInfo {
            head_sha: None,
            short_sha: None,
            branch: None,
        },
        built_at_iso: "2026-01-31T00:00:00Z".to_string(),
        global_excludes: GlobalExcludes::default(),
    };

    let result = write_bundle(&plan).unwrap();
    assert!(result.bytes_written > 0);

    let file = std::fs::File::open(&output_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    assert!(archive.by_name("README.md").is_ok());
    assert!(archive.by_name("app/src/main.ts").is_ok());
    assert!(archive.by_name("dist/bundle.js").is_err());

    let mut manifest = archive.by_name("BUNDLE_MANIFEST.json").unwrap();
    let mut manifest_content = String::new();
    manifest.read_to_string(&mut manifest_content).unwrap();
    assert!(manifest_content.contains("\"repoId\""));
    assert!(manifest_content.contains("\"presetId\""));
    assert!(manifest_content.contains("\"fileCount\""));
    assert!(manifest_content.contains("\"totalBytesBestEffort\""));
}

#[test]
fn progress_callbacks_follow_phase_order() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    std::fs::write(repo_root.join("README.md"), "root").unwrap();

    let output_path = repo_root.join("bundle.zip");
    let plan = BundlePlan {
        output_path: output_path.clone(),
        repo_root: repo_root.to_path_buf(),
        repo_id: "repo".to_string(),
        preset_id: "full".to_string(),
        preset_name: "Full".to_string(),
        selection: BundleSelection {
            include_root: true,
            top_level_dirs: vec![],
            excluded_subdirs: vec![],
        },
        git: BundleGitInfo {
            head_sha: None,
            short_sha: None,
            branch: None,
        },
        built_at_iso: "2026-01-31T00:00:00Z".to_string(),
        global_excludes: GlobalExcludes::default(),
    };

    let phases: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let phases_clone = Arc::clone(&phases);
    let sink = CallbackProgressSink::new(move |message| {
        let phase = match message {
            ProgressMessage::Progress { phase, .. } => phase.to_string(),
            ProgressMessage::Done { .. } => "done".to_string(),
        };
        phases_clone.lock().unwrap().push(phase);
    });

    write_bundle_with_progress(&plan, Box::new(sink)).unwrap();

    let phases = phases.lock().unwrap();
    let scanning_index = phases.iter().position(|phase| phase == "scanning").unwrap();
    let zipping_index = phases.iter().position(|phase| phase == "zipping").unwrap();
    let finalizing_index = phases
        .iter()
        .position(|phase| phase == "finalizing")
        .unwrap();
    let syncing_index = phases.iter().position(|phase| phase == "syncing").unwrap();
    let done_index = phases.iter().position(|phase| phase == "done").unwrap();

    assert!(scanning_index < zipping_index);
    assert!(zipping_index < finalizing_index);
    assert!(finalizing_index < syncing_index);
    assert!(syncing_index < done_index);
}
