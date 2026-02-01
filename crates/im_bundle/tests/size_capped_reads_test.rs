// Path: crates/im_bundle/tests/size_capped_reads_test.rs
// Description: Ensures bundle writes only the bytes present at file-open time even if file grows

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::thread;
use std::time::Duration;

use im_bundle::plan::{BundleGitInfo, BundleSelection, GlobalExcludes};
use im_bundle::writer::write_bundle;
use im_bundle::plan::BundlePlan;
use tempfile::tempdir;

#[test]
fn caps_file_reads_to_initial_length() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();

    let initial_size = 16 * 1024 * 1024;
    let original_bytes = vec![b'a'; initial_size];
    // Use .dat extension to avoid matching any exclude lists.
    let file_path = repo_root.join("data.dat");
    fs::write(&file_path, &original_bytes).unwrap();

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

    let append_handle = thread::spawn({
        let file_path = file_path.clone();
        move || {
            thread::sleep(Duration::from_millis(10));
            let mut file = OpenOptions::new().append(true).open(&file_path).unwrap();
            file.write_all(b"EXTRA").unwrap();
            file.flush().unwrap();
        }
    });

    let result = write_bundle(&plan).unwrap();
    assert!(result.bytes_written > 0);

    append_handle.join().unwrap();

    let file = File::open(&output_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut entry = archive.by_name("data.dat").unwrap();
    let mut contents = Vec::new();
    entry.read_to_end(&mut contents).unwrap();

    assert_eq!(contents.len(), original_bytes.len());
    assert_eq!(contents, original_bytes);
}
