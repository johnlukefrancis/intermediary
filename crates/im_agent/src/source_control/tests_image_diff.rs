// Path: crates/im_agent/src/source_control/tests_image_diff.rs
// Description: Real-git tempdir tests for before/after image-diff side selection

use base64::{engine::general_purpose::STANDARD, Engine as _};

use crate::protocol::{ImageDiffSide, ImageDiffSource, SourceControlArea};

use super::runner::IMAGE_DIFF_SIDE_LIMIT;
use super::source_control_image_diff;
use super::tests_support::*;

/// A tiny but real PNG header plus a per-case tail, so every side carries
/// distinguishable binary bytes that must survive base64 unchanged.
fn png(tail: &[u8]) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.extend_from_slice(tail);
    bytes
}

fn side(side: &Option<ImageDiffSide>) -> &ImageDiffSide {
    side.as_ref().expect("side present")
}

fn assert_side(actual: &Option<ImageDiffSide>, source: ImageDiffSource, bytes: &[u8]) {
    let actual = side(actual);
    assert_eq!(actual.source, source);
    assert_eq!(actual.mime_type, "image/png");
    assert_eq!(actual.data_base64, STANDARD.encode(bytes));
    assert_eq!(actual.bytes, bytes.len() as u64);
    assert!(!actual.truncated);
}

#[tokio::test]
async fn worktree_diff_pairs_index_bytes_with_disk_bytes() {
    let (_temp, root) = init_repo_with_commit();
    let staged = png(b"staged");
    let edited = png(b"edited on disk");
    write(&root, "art/logo.png", &staged);
    git(&root, &["add", "art/logo.png"]);
    write(&root, "art/logo.png", &edited);

    let diff = source_control_image_diff(
        &root,
        "art/logo.png",
        None,
        SourceControlArea::Worktree,
        None,
    )
    .await
    .expect("worktree image diff");

    assert_side(&diff.before, ImageDiffSource::Index, &staged);
    assert_side(&diff.after, ImageDiffSource::Worktree, &edited);
}

#[tokio::test]
async fn index_diff_pairs_head_bytes_with_index_bytes() {
    let (_temp, root) = init_repo_with_commit();
    let committed = png(b"committed");
    let staged = png(b"staged");
    write(&root, "art/logo.png", &committed);
    git(&root, &["add", "art/logo.png"]);
    git(&root, &["commit", "-qm", "add image"]);
    write(&root, "art/logo.png", &staged);
    git(&root, &["add", "art/logo.png"]);

    let diff =
        source_control_image_diff(&root, "art/logo.png", None, SourceControlArea::Index, None)
            .await
            .expect("index image diff");

    assert_side(&diff.before, ImageDiffSource::Head, &committed);
    assert_side(&diff.after, ImageDiffSource::Index, &staged);
}

#[tokio::test]
async fn untracked_image_has_no_previous_side() {
    let (_temp, root) = init_repo_with_commit();
    let added = png(b"brand new");
    write(&root, "new.png", &added);

    let diff = source_control_image_diff(&root, "new.png", None, SourceControlArea::Worktree, None)
        .await
        .expect("untracked image diff");

    assert!(diff.before.is_none());
    assert_side(&diff.after, ImageDiffSource::Worktree, &added);
}

#[tokio::test]
async fn intent_to_add_image_has_no_previous_side() {
    let (_temp, root) = init_repo_with_commit();
    let added = png(b"intent to add");
    write(&root, "new.png", &added);
    git(&root, &["add", "-N", "new.png"]);

    let diff = source_control_image_diff(&root, "new.png", None, SourceControlArea::Worktree, None)
        .await
        .expect("intent-to-add image diff");

    assert!(diff.before.is_none(), "empty staged blob is not a picture");
    assert_side(&diff.after, ImageDiffSource::Worktree, &added);
}

#[tokio::test]
async fn image_deleted_on_disk_has_no_current_side() {
    let (_temp, root) = init_repo_with_commit();
    let committed = png(b"committed");
    write(&root, "art/logo.png", &committed);
    git(&root, &["add", "art/logo.png"]);
    git(&root, &["commit", "-qm", "add image"]);
    std::fs::remove_file(root.join("art/logo.png")).expect("remove image");

    let diff = source_control_image_diff(
        &root,
        "art/logo.png",
        None,
        SourceControlArea::Worktree,
        None,
    )
    .await
    .expect("deleted worktree image diff");

    assert_side(&diff.before, ImageDiffSource::Index, &committed);
    assert!(diff.after.is_none());
}

#[tokio::test]
async fn image_deleted_in_index_keeps_head_side_only() {
    let (_temp, root) = init_repo_with_commit();
    let committed = png(b"committed");
    write(&root, "art/logo.png", &committed);
    git(&root, &["add", "art/logo.png"]);
    git(&root, &["commit", "-qm", "add image"]);
    git(&root, &["rm", "-q", "art/logo.png"]);

    let diff =
        source_control_image_diff(&root, "art/logo.png", None, SourceControlArea::Index, None)
            .await
            .expect("staged delete image diff");

    assert_side(&diff.before, ImageDiffSource::Head, &committed);
    assert!(diff.after.is_none(), "no stage 0 after a staged delete");
}

#[tokio::test]
async fn renamed_image_reads_its_previous_path_from_head() {
    let (_temp, root) = init_repo_with_commit();
    let committed = png(b"committed");
    write(&root, "art/old.png", &committed);
    git(&root, &["add", "art/old.png"]);
    git(&root, &["commit", "-qm", "add image"]);
    git(&root, &["mv", "art/old.png", "art/new.png"]);

    let diff = source_control_image_diff(
        &root,
        "art/new.png",
        Some("art/old.png"),
        SourceControlArea::Index,
        None,
    )
    .await
    .expect("renamed image diff");

    assert_side(&diff.before, ImageDiffSource::Head, &committed);
    assert_side(&diff.after, ImageDiffSource::Index, &committed);
}

#[tokio::test]
async fn conflicted_image_reports_ours_and_theirs_stages() {
    let (_temp, root) = init_repo_with_commit();
    let base = png(b"base");
    write(&root, "art/logo.png", &base);
    git(&root, &["add", "art/logo.png"]);
    git(&root, &["commit", "-qm", "add image"]);

    let ours = png(b"ours");
    write(&root, "art/logo.png", &ours);
    git(&root, &["commit", "-qam", "ours"]);

    git(&root, &["checkout", "-q", "-b", "theirs", "HEAD~1"]);
    let theirs = png(b"theirs");
    write(&root, "art/logo.png", &theirs);
    git(&root, &["commit", "-qam", "theirs"]);
    git(&root, &["checkout", "-q", "main"]);
    assert!(
        !git_succeeds(&root, &["merge", "theirs"]),
        "merge must conflict"
    );

    let diff = source_control_image_diff(
        &root,
        "art/logo.png",
        None,
        SourceControlArea::Worktree,
        None,
    )
    .await
    .expect("conflicted image diff");

    assert_side(&diff.before, ImageDiffSource::Ours, &ours);
    assert_side(&diff.after, ImageDiffSource::Theirs, &theirs);
}

#[tokio::test]
async fn unborn_branch_has_no_head_side() {
    let (_temp, root) = init_repo();
    let added = png(b"first ever");
    write(&root, "first.png", &added);
    git(&root, &["add", "first.png"]);

    let diff = source_control_image_diff(&root, "first.png", None, SourceControlArea::Index, None)
        .await
        .expect("unborn index image diff");

    assert!(diff.before.is_none(), "HEAD does not resolve yet");
    assert_side(&diff.after, ImageDiffSource::Index, &added);
}

#[tokio::test]
async fn oversized_sides_report_the_bound_instead_of_bytes() {
    let (_temp, root) = init_repo_with_commit();
    let big = png(&vec![7_u8; IMAGE_DIFF_SIDE_LIMIT]);
    write(&root, "big.png", &big);
    git(&root, &["add", "big.png"]);
    write(&root, "big.png", &png(&vec![9_u8; IMAGE_DIFF_SIDE_LIMIT]));

    let diff = source_control_image_diff(&root, "big.png", None, SourceControlArea::Worktree, None)
        .await
        .expect("oversized image diff");

    for (actual, source) in [
        (&diff.before, ImageDiffSource::Index),
        (&diff.after, ImageDiffSource::Worktree),
    ] {
        let actual = side(actual);
        assert_eq!(actual.source, source);
        assert!(actual.truncated);
        assert!(actual.data_base64.is_empty());
        assert_eq!(actual.bytes, IMAGE_DIFF_SIDE_LIMIT as u64);
    }
}

#[tokio::test]
async fn unsupported_extensions_are_refused_before_git_runs() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "scan.tiff", b"not previewable");

    let error =
        source_control_image_diff(&root, "scan.tiff", None, SourceControlArea::Worktree, None)
            .await
            .expect_err("unsupported extension");
    assert_eq!(error.code(), "UNSUPPORTED_IMAGE_FILE");
}

#[tokio::test]
async fn image_diff_rejects_invalid_paths_before_running_git() {
    let (_temp, root) = init_repo_with_commit();
    let error =
        source_control_image_diff(&root, "../logo.png", None, SourceControlArea::Worktree, None)
            .await
            .expect_err("traversal rejected");
    assert_eq!(error.code(), "INVALID_PATH");
}

#[tokio::test]
async fn subdirectory_root_resolves_blob_specs_against_its_own_root() {
    let (_temp, root) = init_repo_with_commit();
    let decoy = png(b"top-level decoy");
    let committed = png(b"sub committed");
    let edited = png(b"sub edited");
    write(&root, "logo.png", &decoy);
    write(&root, "sub/logo.png", &committed);
    git(&root, &["add", "logo.png", "sub/logo.png"]);
    git(&root, &["commit", "-qm", "images"]);
    write(&root, "sub/logo.png", &edited);

    let sub = root.join("sub");
    let worktree = source_control_image_diff(&sub, "logo.png", None, SourceControlArea::Worktree, None)
        .await
        .expect("worktree image diff below the top level");
    assert_eq!(STANDARD.decode(&side(&worktree.before).data_base64).expect("b64"), committed);
    assert_eq!(STANDARD.decode(&side(&worktree.after).data_base64).expect("b64"), edited);

    git(&root, &["add", "sub/logo.png"]);
    let index = source_control_image_diff(&sub, "logo.png", None, SourceControlArea::Index, None)
        .await
        .expect("index image diff below the top level");
    assert!(matches!(side(&index.before).source, ImageDiffSource::Head));
    assert_eq!(STANDARD.decode(&side(&index.before).data_base64).expect("b64"), committed);
    assert_eq!(STANDARD.decode(&side(&index.after).data_base64).expect("b64"), edited);
}

