// Path: crates/im_agent/src/repos/image_file_reader_tests.rs
// Description: Image preview reader tests - mime gate, root escape, size bounds (process-wide and per-request), one-revision binding

use super::{
    read_bounded, read_image_file, read_image_file_bounded, size_gate, verify_still, ImageStamp,
    MAX_IMAGE_FILE_BYTES,
};
use std::fs;
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs as unix_fs;

fn repo_root() -> TempDir {
    let root = TempDir::new().expect("tempdir");
    fs::create_dir_all(root.path().join("docs/images")).expect("image dir");
    root
}

#[tokio::test]
async fn reads_supported_image_file() {
    let root = repo_root();
    fs::write(root.path().join("docs/images/capture.png"), b"png bytes").expect("write png");

    let result = read_image_file(
        root.path().to_str().expect("root path"),
        "docs/images/capture.png",
    )
    .await
    .expect("read image");

    assert_eq!(result.mime_type, "image/png");
    assert_eq!(result.data_base64, "cG5nIGJ5dGVz");
    assert_eq!(result.bytes, 9);
    assert!(result.mtime_ms > 0);
}

#[tokio::test]
async fn rejects_traversal_paths() {
    let root = repo_root();
    let err = read_image_file(root.path().to_str().expect("root path"), "../outside.png")
        .await
        .expect_err("traversal should fail");

    assert_eq!(err.code(), "INVALID_PATH");
}

#[cfg(unix)]
#[tokio::test]
async fn rejects_symlink_file_outside_repo_root() {
    let root = repo_root();
    let outside = TempDir::new().expect("outside tempdir");
    fs::write(outside.path().join("secret.png"), b"outside image").expect("write outside");
    unix_fs::symlink(
        outside.path().join("secret.png"),
        root.path().join("docs/images/link.png"),
    )
    .expect("symlink file");

    let err = read_image_file(
        root.path().to_str().expect("root path"),
        "docs/images/link.png",
    )
    .await
    .expect_err("outside symlink should fail");

    assert_eq!(err.code(), "INVALID_PATH");
}

#[tokio::test]
async fn reports_missing_files() {
    let root = repo_root();
    let err = read_image_file(
        root.path().to_str().expect("root path"),
        "docs/images/missing.png",
    )
    .await
    .expect_err("missing file should fail");

    assert_eq!(err.code(), "FILE_NOT_FOUND");
}

#[tokio::test]
async fn rejects_directory_paths() {
    let root = repo_root();
    let err = read_image_file(root.path().to_str().expect("root path"), "docs/images")
        .await
        .expect_err("directory should fail");

    assert_eq!(err.code(), "UNSUPPORTED_IMAGE_FILE");
}

#[tokio::test]
async fn rejects_unsupported_extensions() {
    let root = repo_root();
    fs::write(root.path().join("docs/images/capture.tiff"), b"tiff bytes").expect("write tiff");

    let err = read_image_file(
        root.path().to_str().expect("root path"),
        "docs/images/capture.tiff",
    )
    .await
    .expect_err("unsupported image should fail");

    assert_eq!(err.code(), "UNSUPPORTED_IMAGE_FILE");
}

#[tokio::test]
async fn rejects_oversized_images() {
    let root = repo_root();
    let bytes = vec![0_u8; (MAX_IMAGE_FILE_BYTES + 1) as usize];
    fs::write(root.path().join("docs/images/large.png"), bytes).expect("write large image");

    let err = read_image_file(
        root.path().to_str().expect("root path"),
        "docs/images/large.png",
    )
    .await
    .expect_err("large image should fail");

    assert_eq!(err.code(), "UNSUPPORTED_IMAGE_FILE");
}

/// The caller's bound is enforced from the stat, before any byte is read,
/// with the message the UI keys on; without a bound only the process-wide
/// ceiling applies.
#[tokio::test]
async fn honours_the_requested_size_bound_before_reading() {
    let root = repo_root();
    fs::write(root.path().join("docs/images/shot.png"), vec![0_u8; 4096]).expect("write png");
    let root_str = root.path().to_str().expect("root path");

    let err = read_image_file_bounded(root_str, "docs/images/shot.png", Some(4095))
        .await
        .expect_err("over the requested bound");
    assert_eq!(err.code(), "UNSUPPORTED_IMAGE_FILE");
    assert_eq!(err.message(), "Image exceeds the requested size bound");

    let exact = read_image_file_bounded(root_str, "docs/images/shot.png", Some(4096))
        .await
        .expect("at the bound reads");
    assert_eq!(exact.bytes, 4096);

    let unbounded = read_image_file_bounded(root_str, "docs/images/shot.png", None)
        .await
        .expect("no bound reads");
    assert_eq!(unbounded.bytes, 4096);
}

/// A stamp that moved across the read - size or mtime - refuses the bytes
/// with the message the UI keys on as `IMAGE CHANGED`; so does a read that
/// saw fewer bytes than the file has now.
#[test]
fn a_moved_stamp_refuses_the_read() {
    let before = ImageStamp {
        len: 4096,
        mtime_ms: 1_757_168_000_000,
    };
    assert!(verify_still(&before, &before, 4096).is_ok());

    let grown = ImageStamp {
        len: 8192,
        ..before
    };
    let err = verify_still(&before, &grown, 4096).expect_err("size moved");
    assert_eq!(err.code(), "UNSUPPORTED_IMAGE_FILE");
    assert_eq!(err.message(), "Image changed while it was being read");

    let rewritten = ImageStamp {
        mtime_ms: before.mtime_ms + 1,
        ..before
    };
    assert!(
        verify_still(&before, &rewritten, 4096).is_err(),
        "mtime moved"
    );
    assert!(
        verify_still(&before, &before, 4095).is_err(),
        "a short read does not match the size on disk"
    );
}

/// A file that grows past the requested bound between the stat and the read
/// transfers one byte of overflow, never the excess, and that byte is what
/// the post-read size gate refuses.
#[tokio::test]
async fn growth_past_the_bound_is_refused_without_transferring_the_excess() {
    let root = repo_root();
    let path = root.path().join("docs/images/growing.png");
    fs::write(&path, vec![0_u8; 4096 + 1000]).expect("write grown png");

    let bytes = read_bounded(&path, 4096).await.expect("bounded read");
    assert_eq!(bytes.len(), 4097, "one byte past the bound, no more");

    let err = size_gate(bytes.len() as u64, Some(4096)).expect_err("over the requested bound");
    assert_eq!(err.code(), "UNSUPPORTED_IMAGE_FILE");
    assert_eq!(err.message(), "Image exceeds the requested size bound");
    assert!(size_gate(4096, Some(4096)).is_ok());
    assert_eq!(
        size_gate(MAX_IMAGE_FILE_BYTES + 1, None)
            .expect_err("over the process-wide ceiling")
            .message(),
        "Image file is too large for the preview"
    );
}
