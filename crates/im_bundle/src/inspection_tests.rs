// Path: crates/im_bundle/src/inspection_tests.rs
// Description: Inspection selection parity, size ranking, bounded scans and containment regressions

use super::*;

fn selection() -> BundleSelection {
    BundleSelection {
        include_root: false,
        top_level_dirs: vec![],
        included_subdirs: vec!["Saved/Keep".into()],
        included_files: vec!["Saved/single.txt".into()],
        excluded_subdirs: vec!["Saved".into()],
        excluded_files: vec!["Saved/Keep/skip.txt".into()],
    }
}

fn write(root: &Path, path: &str, bytes: usize) {
    let target = root.join(path);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(target, vec![b'x'; bytes]).unwrap();
}

#[test]
fn reports_total_and_selected_bytes_through_excluded_ancestors() {
    let repo = tempfile::tempdir().unwrap();
    for (path, bytes) in [
        ("Saved/Keep/large.txt", 4096),
        ("Saved/Keep/small.txt", 128),
        ("Saved/single.txt", 256),
        ("Saved/Keep/skip.txt", 800),
        ("Saved/Keep/global.tmp", 1600),
        ("Saved/Other/hidden.txt", 3200),
        ("Saved/.git/config", 6400),
    ] {
        write(repo.path(), path, bytes);
    }
    let excludes = GlobalExcludes::default();
    let result = inspect_entry(
        repo.path(),
        "Saved",
        &selection(),
        &excludes,
        &BundleCancelToken::new(),
    )
    .unwrap();
    assert!(result.complete);
    assert_eq!((result.bytes, result.file_count), (10080, 6));
    assert_eq!(
        (result.included_bytes, result.included_file_count),
        (4480, 3)
    );
    assert_eq!(
        result
            .largest_included_files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        [
            "Saved/Keep/large.txt",
            "Saved/single.txt",
            "Saved/Keep/small.txt"
        ]
    );
    let file = inspect_entry(
        repo.path(),
        "Saved/Keep/skip.txt",
        &selection(),
        &excludes,
        &BundleCancelToken::new(),
    )
    .unwrap();
    assert_eq!((file.bytes, file.included_bytes), (800, 0));
    assert!(!file.is_directory);
    assert!(file.modified_ms.is_some());
}

#[test]
fn largest_files_are_bounded_and_partial_results_are_explicit() {
    let repo = tempfile::tempdir().unwrap();
    for i in 1..=8 {
        write(repo.path(), &format!("Saved/Keep/{i}.txt"), i * 10);
    }
    let excludes = GlobalExcludes::default();
    let cancel = BundleCancelToken::new();
    let result = inspect_entry(repo.path(), "Saved", &selection(), &excludes, &cancel).unwrap();
    assert_eq!(
        result
            .largest_included_files
            .iter()
            .map(|file| file.bytes)
            .collect::<Vec<_>>(),
        [80, 70, 60, 50, 40]
    );
    let partial = inspect_with_limits(
        repo.path(),
        "Saved",
        &selection(),
        &excludes,
        &cancel,
        2,
        MAX_DURATION,
    )
    .unwrap();
    assert!(!partial.complete);
    assert!(partial.bytes < result.bytes);
    let timed = inspect_with_limits(
        repo.path(),
        "Saved",
        &selection(),
        &excludes,
        &cancel,
        MAX_ENTRIES,
        Duration::ZERO,
    )
    .unwrap();
    assert!(!timed.complete);
    assert_eq!(timed.file_count, 0);
    cancel.cancel();
    assert!(matches!(
        inspect_entry(repo.path(), "Saved", &selection(), &excludes, &cancel),
        Err(BundleError::Cancelled)
    ));
}

#[test]
fn rejects_paths_outside_the_tree_and_git_metadata() {
    let repo = tempfile::tempdir().unwrap();
    for path in [
        "../outside",
        "/absolute",
        "Saved/../../outside",
        "Saved\\outside",
        ".git",
        "",
    ] {
        assert!(
            inspect_entry(
                repo.path(),
                path,
                &selection(),
                &GlobalExcludes::default(),
                &BundleCancelToken::new()
            )
            .is_err(),
            "{path}"
        );
    }
}

#[cfg(unix)]
#[test]
fn skips_symlinks_and_rejects_linked_ancestors() {
    let repo = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    write(repo.path(), "Saved/Keep/local.txt", 12);
    write(outside.path(), "external.txt", 100);
    std::os::unix::fs::symlink(outside.path(), repo.path().join("Saved/link")).unwrap();
    let excludes = GlobalExcludes::default();
    let cancel = BundleCancelToken::new();
    let result = inspect_entry(repo.path(), "Saved", &selection(), &excludes, &cancel).unwrap();
    assert_eq!(result.bytes, 12);
    assert!(inspect_entry(
        repo.path(),
        "Saved/link/external.txt",
        &selection(),
        &excludes,
        &cancel
    )
    .is_err());
}
