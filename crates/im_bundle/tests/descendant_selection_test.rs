// Path: crates/im_bundle/tests/descendant_selection_test.rs
// Description: Archive and Git evidence stay bounded to explicit descendants of excluded folders

use std::io::Read;
use std::path::Path;
use std::process::Command;

use im_bundle::plan::{BundlePlan, BundleSelection, GlobalExcludes};
use im_bundle::write_bundle;
use tempfile::tempdir;

fn git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(repo)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn only_explicit_descendants_enter_archive_and_git_patch() {
    let root = tempdir().unwrap();
    let repo = root.path().join("repo");
    let paths = [
        "README.md",
        "other.md",
        "Saved/Keep/chosen.txt",
        "Saved/Keep/skip.txt",
        "Saved/Keep/Noise/hidden.txt",
        "Saved/Keep/Noise/Again/chosen.txt",
        "Saved/Logs/one.txt",
        "Saved/Logs/deleted.txt",
        "Saved/Logs/two.txt",
        "Saved/other.txt",
        "Saved/Keep/target/cache.txt",
        "Saved/Keep/blob.bin",
        "Saved/private/hidden.txt",
    ];
    for path in paths {
        let file = repo.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, "before\n").unwrap();
    }
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "user.name", "Selection Test"]);
    git(&repo, &["config", "user.email", "selection@example.test"]);
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "baseline"]);
    for path in paths {
        std::fs::write(repo.join(path), "after\n").unwrap();
    }
    std::fs::remove_file(repo.join("Saved/Logs/deleted.txt")).unwrap();

    let output = root.path().join("bundle.zip");
    let plan = BundlePlan {
        output_path: output.clone(),
        repo_root: repo,
        repo_id: "test".into(),
        preset_id: "context".into(),
        preset_name: "Context".into(),
        built_at_iso: "2026-10-10T00:00:00Z".into(),
        selection: BundleSelection {
            include_root: false,
            top_level_dirs: vec![],
            included_subdirs: vec!["Saved/Keep".into(), "Saved/Keep/Noise/Again".into()],
            included_files: vec![
                "README.md".into(),
                "Saved/Logs/one.txt".into(),
                "Saved/Logs/deleted.txt".into(),
                "Saved/Keep/blob.bin".into(),
                "Saved/private/hidden.txt".into(),
            ],
            excluded_subdirs: vec!["Saved".into(), "Saved/Keep/Noise".into()],
            excluded_files: vec!["Saved/Keep/skip.txt".into()],
        },
        global_excludes: GlobalExcludes {
            dir_names: vec!["Saved".into(), "target".into()],
            dir_suffixes: vec![],
            file_names: vec![],
            extensions: vec![".bin".into()],
            patterns: vec!["private".into()],
        },
    };
    write_bundle(&plan).unwrap();
    let mut archive = zip::ZipArchive::new(std::fs::File::open(output).unwrap()).unwrap();
    let mut ordinary: Vec<_> = archive
        .file_names()
        .filter(|name| !name.starts_with("BUNDLE_"))
        .map(str::to_string)
        .collect();
    ordinary.sort();
    assert_eq!(
        ordinary,
        [
            "README.md",
            "Saved/Keep/Noise/Again/chosen.txt",
            "Saved/Keep/chosen.txt",
            "Saved/Logs/one.txt"
        ]
    );
    let mut patch = String::new();
    archive
        .by_name("BUNDLE_GIT_DIFF.patch")
        .unwrap()
        .read_to_string(&mut patch)
        .unwrap();
    for path in &ordinary {
        assert!(patch.contains(path), "missing {path}");
    }
    assert!(patch.contains("Saved/Logs/deleted.txt"));
    for path in paths {
        if !ordinary.iter().any(|selected| selected == path) && path != "Saved/Logs/deleted.txt" {
            assert!(!patch.contains(path), "leaked {path}");
        }
    }
    let mut manifest = String::new();
    archive
        .by_name("BUNDLE_MANIFEST.json")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    assert_eq!(manifest["git"]["status"], "complete");
    assert_eq!(
        manifest["selection"]["topLevelDirsIncluded"],
        serde_json::json!([])
    );
    assert_eq!(
        manifest["selection"]["includedFiles"],
        serde_json::json!(plan.selection.included_files)
    );
}
