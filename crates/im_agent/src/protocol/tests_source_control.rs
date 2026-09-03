// Path: crates/im_agent/src/protocol/tests_source_control.rs
// Description: Wire-shape tests for the source-control command and status payloads

use serde_json::json;

use super::commands_source_control::{
    SourceControlActionCommand, SourceControlActionKind, SourceControlActionPayload,
    SourceControlDiscardTarget, SourceControlScope, SourceControlWorktreeStamp,
};
use super::responses_source_control::{
    SourceControlActionResult, SourceControlChange, SourceControlEntry, SourceControlEntryArea,
    SourceControlOmitted, SourceControlStatus,
};

fn action_json(payload: &SourceControlActionPayload) -> serde_json::Value {
    serde_json::to_value(SourceControlActionCommand {
        repo_id: "repo".to_string(),
        action: payload.clone(),
    })
    .expect("serialize action")
}

fn round_trip(payload: SourceControlActionPayload) -> SourceControlActionPayload {
    let wire = serde_json::to_string(&payload).expect("serialize");
    serde_json::from_str(&wire).expect("deserialize")
}

fn stamp(bytes: u64, mtime_ms: i64, mtime_nanos: u32) -> SourceControlWorktreeStamp {
    SourceControlWorktreeStamp {
        bytes,
        mtime_ms,
        mtime_nanos,
    }
}

#[test]
fn discard_carries_targets_with_optional_stamps_and_expected_missing() {
    let payload = SourceControlActionPayload::Discard {
        targets: vec![
            SourceControlDiscardTarget {
                path: "copy.txt".to_string(),
                expected_stamp: Some(stamp(12, 1_757_000_000_123, 456_000)),
                expected_missing: false,
            },
            SourceControlDiscardTarget {
                path: "gone.txt".to_string(),
                expected_stamp: None,
                expected_missing: true,
            },
        ],
    };
    assert_eq!(
        action_json(&payload),
        json!({
            "repoId": "repo",
            "action": {
                "kind": "discard",
                "targets": [
                    {
                        "path": "copy.txt",
                        "expectedStamp": {
                            "bytes": 12,
                            "mtimeMs": 1_757_000_000_123i64,
                            "mtimeNanos": 456_000
                        }
                    },
                    { "path": "gone.txt", "expectedMissing": true }
                ]
            }
        })
    );
    assert_eq!(round_trip(payload.clone()), payload);
}

#[test]
fn commit_carries_the_reviewed_index_tree_and_head_null_on_an_unborn_branch() {
    let payload = SourceControlActionPayload::Commit {
        message: "Change base".to_string(),
        expected_index_tree_sha: "4b825dc642cb6eb9a060e54bf8d69288fbee4904".to_string(),
        expected_head_sha: None,
    };
    assert_eq!(
        action_json(&payload),
        json!({
            "repoId": "repo",
            "action": {
                "kind": "commit",
                "message": "Change base",
                "expectedIndexTreeSha": "4b825dc642cb6eb9a060e54bf8d69288fbee4904",
                "expectedHeadSha": null
            }
        })
    );
    assert_eq!(round_trip(payload.clone()), payload);

    let payload = SourceControlActionPayload::Commit {
        message: "Change base".to_string(),
        expected_index_tree_sha: "4b825dc642cb6eb9a060e54bf8d69288fbee4904".to_string(),
        expected_head_sha: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
    };
    assert_eq!(
        action_json(&payload)["action"]["expectedHeadSha"],
        json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    );
    assert_eq!(round_trip(payload.clone()), payload);
}

#[test]
fn stage_all_names_the_section_without_a_pathspec() {
    let payload = SourceControlActionPayload::Stage {
        scope: SourceControlScope::All,
    };
    assert_eq!(
        action_json(&payload),
        json!({ "repoId": "repo", "action": { "kind": "stage", "scope": { "mode": "all" } } })
    );
    assert_eq!(round_trip(payload.clone()), payload);
}

#[test]
fn status_carries_the_index_identity_the_lock_state_and_entry_stamps() {
    let status = SourceControlStatus {
        branch: Some("main".to_string()),
        head_sha: None,
        detached: false,
        upstream: None,
        ahead: None,
        behind: None,
        index: vec![SourceControlEntry {
            path: "staged.txt".to_string(),
            original_path: None,
            area: SourceControlEntryArea::Index,
            change: SourceControlChange::Added,
            worktree_stamp: None,
            worktree_missing: false,
        }],
        worktree: vec![
            SourceControlEntry {
                path: "edited.txt".to_string(),
                original_path: None,
                area: SourceControlEntryArea::Worktree,
                change: SourceControlChange::Modified,
                worktree_stamp: Some(stamp(7, 42, 9)),
                worktree_missing: false,
            },
            SourceControlEntry {
                path: "deleted.txt".to_string(),
                original_path: None,
                area: SourceControlEntryArea::Worktree,
                change: SourceControlChange::Deleted,
                worktree_stamp: None,
                worktree_missing: true,
            },
        ],
        conflicts: Vec::new(),
        committable: true,
        index_tree_sha: "abc".to_string(),
        mutation_in_progress: false,
        omitted: SourceControlOmitted::default(),
        truncated: false,
        captured_at_iso: "2026-09-03T00:00:00.000Z".to_string(),
    };
    let wire = serde_json::to_value(&status).expect("serialize status");
    assert_eq!(wire["indexTreeSha"], json!("abc"));
    assert_eq!(wire["mutationInProgress"], json!(false));
    assert_eq!(wire["index"][0].get("worktreeStamp"), None);
    assert_eq!(wire["index"][0].get("worktreeMissing"), None, "false is omitted");
    assert_eq!(
        wire["worktree"][0]["worktreeStamp"],
        json!({ "bytes": 7, "mtimeMs": 42, "mtimeNanos": 9 })
    );
    assert_eq!(wire["worktree"][0].get("worktreeMissing"), None, "false is omitted");
    assert_eq!(wire["worktree"][1]["worktreeMissing"], json!(true));
    assert_eq!(wire["worktree"][1].get("worktreeStamp"), None);
    let back: SourceControlStatus = serde_json::from_value(wire).expect("deserialize status");
    assert_eq!(back, status);
}

#[test]
fn action_result_omits_hook_changed_paths_when_empty_and_carries_them_when_not() {
    let base = SourceControlActionResult {
        repo_id: "repo".to_string(),
        kind: SourceControlActionKind::Commit,
        status: minimal_status(),
        commit_sha: Some("deadbeef".to_string()),
        hook_changed_paths: Vec::new(),
    };
    let wire = serde_json::to_value(&base).expect("serialize result");
    assert_eq!(wire.get("hookChangedPaths"), None);

    let with_hook_changes = SourceControlActionResult {
        hook_changed_paths: vec!["formatted.txt".to_string()],
        ..base
    };
    let wire = serde_json::to_value(&with_hook_changes).expect("serialize result");
    assert_eq!(wire["hookChangedPaths"], json!(["formatted.txt"]));
    let back: SourceControlActionResult = serde_json::from_value(wire).expect("deserialize result");
    assert_eq!(back.hook_changed_paths, vec!["formatted.txt".to_string()]);
}

fn minimal_status() -> SourceControlStatus {
    SourceControlStatus {
        branch: Some("main".to_string()),
        head_sha: Some("deadbeef".to_string()),
        detached: false,
        upstream: None,
        ahead: None,
        behind: None,
        index: Vec::new(),
        worktree: Vec::new(),
        conflicts: Vec::new(),
        committable: false,
        index_tree_sha: "abc".to_string(),
        mutation_in_progress: false,
        omitted: SourceControlOmitted::default(),
        truncated: false,
        captured_at_iso: "2026-09-03T00:00:00.000Z".to_string(),
    }
}
