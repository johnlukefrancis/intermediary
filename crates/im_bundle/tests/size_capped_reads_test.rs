// Path: crates/im_bundle/tests/size_capped_reads_test.rs
// Description: Ensures bundle writes only the bytes present at file-open time even if file grows

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use im_bundle::plan::BundlePlan;
use im_bundle::plan::{BundleSelection, GlobalExcludes};
use im_bundle::progress::ProgressMessage;
use im_bundle::progress_sink::CallbackProgressSink;
use im_bundle::writer::write_bundle_with_progress;
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
            excluded_files: vec![],
        },
        built_at_iso: "2026-01-31T00:00:00Z".to_string(),
        global_excludes: GlobalExcludes::default(),
    };

    let appended = Arc::new(AtomicBool::new(false));
    let sink = CallbackProgressSink::new({
        let file_path = file_path.clone();
        let appended = Arc::clone(&appended);
        move |message| {
            let should_append = matches!(
                message,
                ProgressMessage::Progress {
                    current_file: Some(ref path),
                    current_bytes_done: Some(0),
                    ..
                } if path == "data.dat"
            );
            if !should_append || appended.swap(true, Ordering::SeqCst) {
                return;
            }
            let mut file = OpenOptions::new().append(true).open(&file_path).unwrap();
            file.write_all(b"EXTRA").unwrap();
            file.flush().unwrap();
        }
    });

    let result = write_bundle_with_progress(&plan, Box::new(sink)).unwrap();
    assert!(result.bytes_written > 0);
    assert!(appended.load(Ordering::SeqCst));

    let file = File::open(&output_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut entry = archive.by_name("data.dat").unwrap();
    let mut contents = Vec::new();
    entry.read_to_end(&mut contents).unwrap();

    assert_eq!(contents.len(), original_bytes.len());
    assert_eq!(contents, original_bytes);
}
