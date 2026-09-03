// Path: crates/im_host_agent/src/wsl/wsl_backend_client/tests_timeouts.rs
// Description: Unit tests for the per-command forward timeout ladder

use std::time::Duration;

use im_agent::protocol::{
    BundleSelection, SourceControlActionCommand, SourceControlActionPayload,
    SourceControlDiffCommand, SourceControlImageDiffCommand, SourceControlScope,
    SourceControlStatusCommand, UiCommand,
};

use super::timeouts::*;

fn action(payload: SourceControlActionPayload) -> UiCommand {
    UiCommand::SourceControlAction(SourceControlActionCommand {
        repo_id: "repo".to_string(),
        action: payload,
    })
}

#[test]
fn build_bundle_uses_extended_timeout_budget() {
    let command = UiCommand::BuildBundle(im_agent::protocol::BuildBundleCommand {
        repo_id: "repo".to_string(),
        preset_id: "context".to_string(),
        build_id: "build_1".to_string(),
        selection: BundleSelection {
            include_root: true,
            top_level_dirs: vec![],
            included_subdirs: vec![],
            excluded_subdirs: vec![],
            excluded_files: vec![],
        },
        global_excludes: None,
    });

    assert_eq!(
        timeout_for_command(&command),
        FORWARD_REQUEST_TIMEOUT_BUILD_BUNDLE
    );
}

#[test]
fn stage_file_uses_default_timeout_budget() {
    let command = UiCommand::StageFile(im_agent::protocol::StageFileCommand {
        repo_id: "repo".to_string(),
        path: "src/main.rs".to_string(),
    });

    assert_eq!(
        timeout_for_command(&command),
        FORWARD_REQUEST_TIMEOUT_DEFAULT
    );
}
/// One assertion per timeout class: the ladder is the contract, and a class
/// silently collapsing into its neighbour is the defect this guards.
#[test]
fn every_source_control_class_keeps_its_own_budget() {
    let status = UiCommand::SourceControlStatus(SourceControlStatusCommand {
        repo_id: "repo".to_string(),
    });
    let diff = UiCommand::SourceControlDiff(SourceControlDiffCommand {
        repo_id: "repo".to_string(),
        path: "src/main.rs".to_string(),
        original_path: None,
        area: im_agent::protocol::SourceControlArea::Worktree,
    });
    let image_diff = UiCommand::SourceControlImageDiff(SourceControlImageDiffCommand {
        repo_id: "repo".to_string(),
        path: "art/logo.png".to_string(),
        original_path: None,
        area: im_agent::protocol::SourceControlArea::Worktree,
    });
    assert_eq!(timeout_for_command(&status), Duration::from_secs(120));
    assert_eq!(timeout_for_command(&diff), Duration::from_secs(120));
    assert_eq!(timeout_for_command(&image_diff), Duration::from_secs(120));

    let stage = action(SourceControlActionPayload::Stage {
        scope: SourceControlScope::All,
    });
    let unstage = action(SourceControlActionPayload::Unstage {
        scope: SourceControlScope::All,
    });
    assert_eq!(timeout_for_command(&stage), Duration::from_secs(280));
    assert_eq!(timeout_for_command(&unstage), Duration::from_secs(280));

    let discard = action(SourceControlActionPayload::Discard { targets: vec![] });
    assert_eq!(timeout_for_command(&discard), Duration::from_secs(340));

    let commit = action(SourceControlActionPayload::Commit {
        message: "message".to_string(),
        expected_snapshot_id: "snapshot".to_string(),
    });
    assert_eq!(timeout_for_command(&commit), Duration::from_secs(380));

    assert_eq!(
        timeout_for_command(&action(SourceControlActionPayload::Push)),
        Duration::from_secs(420)
    );
    assert_eq!(
        timeout_for_command(&action(SourceControlActionPayload::Pull)),
        Duration::from_secs(420)
    );
}

/// The ladder must rise strictly by class, so a longer action never expires
/// before a shorter one that runs a subset of its Git commands.
#[test]
fn the_source_control_ladder_is_strictly_increasing() {
    assert!(
        FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_READ
            < FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_INDEX
            && FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_INDEX
                < FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_DISCARD
            && FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_DISCARD
                < FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_COMMIT
            && FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_COMMIT
                < FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_REMOTE
    );
}

/// Shutdown sits above the WSL agent's emergency drain bound so a single
/// forward attempt can cover the whole thing.
#[test]
fn shutdown_uses_the_drain_aware_budget() {
    assert_eq!(
        timeout_for_command(&UiCommand::Shutdown),
        Duration::from_secs(470)
    );
    assert!(FORWARD_REQUEST_TIMEOUT_SHUTDOWN > im_agent::server::SHUTDOWN_EMERGENCY_BOUND);
}
