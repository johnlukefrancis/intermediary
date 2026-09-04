// Path: crates/im_bundle/src/fs_atomic.rs
// Description: Rename that refuses to replace an existing destination, on the two platforms the product runs on

use std::io::{Error, ErrorKind, Result};
use std::path::Path;

/// Moves `from` to `to` only when nothing is at `to`: the rename either lands
/// on empty ground or fails, and it never destroys whatever is already there.
/// `std::fs::rename` silently replaces the destination on both platforms,
/// which is exactly wrong when the file being moved is the only copy of
/// something and someone else may have written the destination in the
/// meantime.
///
/// A destination that exists is `ErrorKind::AlreadyExists`, and `from` is left
/// where it was. A filesystem with no no-replace rename at all is
/// `ErrorKind::Unsupported`, with a message naming that limitation — WSL's
/// mount of a Windows drive is one of those: it rejects the flag with `EINVAL`
/// whether or not the destination exists. Every other failure keeps its own
/// kind (a missing parent directory, a permission denial, a cross-volume
/// move), so callers can tell those apart.
#[cfg(target_os = "linux")]
pub fn rename_no_replace(from: &Path, to: &Path) -> Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let from_c = CString::new(from.as_os_str().as_bytes()).map_err(interior_nul)?;
    let to_c = CString::new(to.as_os_str().as_bytes()).map_err(interior_nul)?;
    // SAFETY: both arguments are NUL-terminated C strings that outlive the
    // call, and `renameat2` only reads through them.
    let outcome = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            from_c.as_ptr(),
            libc::AT_FDCWD,
            to_c.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if outcome == 0 {
        return Ok(());
    }
    Err(classify(Error::last_os_error()))
}

/// `EINVAL`, `ENOSYS` and `ENOTSUP`/`EOPNOTSUPP` all say the same thing here:
/// this kernel or this filesystem has no no-replace rename. That is a distinct
/// answer from "the destination exists", because the caller must not respond
/// to it by falling back to a rename that replaces.
#[cfg(target_os = "linux")]
fn classify(error: Error) -> Error {
    const UNSUPPORTED: [i32; 4] = [libc::EINVAL, libc::ENOSYS, libc::ENOTSUP, libc::EOPNOTSUPP];
    match error.raw_os_error() {
        Some(code) if UNSUPPORTED.contains(&code) => Error::new(
            ErrorKind::Unsupported,
            format!("this filesystem cannot rename without replacing the destination: {error}"),
        ),
        _ => error,
    }
}

#[cfg(target_os = "linux")]
fn interior_nul(error: std::ffi::NulError) -> Error {
    Error::new(ErrorKind::InvalidInput, format!("path contains a NUL byte: {error}"))
}

#[cfg(windows)]
pub fn rename_no_replace(from: &Path, to: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;

    let from_w: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to_w: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: both arguments are NUL-terminated UTF-16 buffers that outlive the
    // call, and `MoveFileExW` only reads through them. No flags are passed, so
    // `MOVEFILE_REPLACE_EXISTING` is absent and an occupied destination fails
    // the call instead of being overwritten.
    let outcome = unsafe { MoveFileExW(from_w.as_ptr(), to_w.as_ptr(), 0) };
    if outcome != 0 {
        return Ok(());
    }
    Err(classify(Error::last_os_error()))
}

/// Windows reports an occupied destination as either of two codes depending on
/// what is sitting there; both are the same answer to this caller.
#[cfg(windows)]
fn classify(error: Error) -> Error {
    use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, ERROR_FILE_EXISTS};

    match error.raw_os_error() {
        Some(code)
            if code as u32 == ERROR_ALREADY_EXISTS || code as u32 == ERROR_FILE_EXISTS =>
        {
            Error::new(
                ErrorKind::AlreadyExists,
                format!("the destination already exists: {error}"),
            )
        }
        _ => error,
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
compile_error!("rename_no_replace is implemented for the Linux and Windows hosts this product runs on");

#[cfg(test)]
mod tests {
    use super::rename_no_replace;

    #[test]
    fn a_rename_onto_empty_ground_moves_the_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let from = temp.path().join("from.txt");
        let to = temp.path().join("to.txt");
        std::fs::write(&from, b"moved\n").expect("source");

        rename_no_replace(&from, &to).expect("the destination is free");

        assert!(!from.exists());
        assert_eq!(std::fs::read(&to).expect("destination"), b"moved\n");
    }

    /// The whole point: an occupied destination is refused rather than
    /// overwritten, and the file being moved stays exactly where it was so the
    /// caller still has both copies to reason about.
    #[test]
    fn an_occupied_destination_is_refused_and_leaves_both_files_standing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let from = temp.path().join("from.txt");
        let to = temp.path().join("to.txt");
        std::fs::write(&from, b"claimed\n").expect("source");
        std::fs::write(&to, b"newer\n").expect("destination");

        let error = rename_no_replace(&from, &to).expect_err("the destination is taken");

        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&from).expect("source"), b"claimed\n");
        assert_eq!(std::fs::read(&to).expect("destination"), b"newer\n");
    }

    /// A whole tree is what a claim and a folder move both hand this function,
    /// so the directory has to arrive with everything under it — one rename,
    /// not a walk.
    #[test]
    fn a_directory_moves_onto_empty_ground_with_its_contents() {
        let temp = tempfile::tempdir().expect("tempdir");
        let from = temp.path().join("tree");
        std::fs::create_dir_all(from.join("deep")).expect("source tree");
        std::fs::write(from.join("deep/a.txt"), b"nested\n").expect("nested file");
        let to = temp.path().join("moved");

        rename_no_replace(&from, &to).expect("the destination is free");

        assert!(!from.exists());
        assert_eq!(
            std::fs::read(to.join("deep/a.txt")).expect("nested file"),
            b"nested\n"
        );
    }

    /// The same refusal for a tree as for a file, and for the same reason: a
    /// replacing rename onto an occupied directory name would take everything
    /// under both names with it.
    #[test]
    fn a_directory_onto_an_occupied_name_is_refused_and_both_trees_stand() {
        let temp = tempfile::tempdir().expect("tempdir");
        let from = temp.path().join("tree");
        std::fs::create_dir_all(&from).expect("source tree");
        std::fs::write(from.join("mine.txt"), b"mine\n").expect("source file");
        let to = temp.path().join("taken");
        std::fs::create_dir_all(&to).expect("destination tree");
        std::fs::write(to.join("theirs.txt"), b"theirs\n").expect("destination file");

        let error = rename_no_replace(&from, &to).expect_err("the destination is taken");

        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(
            std::fs::read(from.join("mine.txt")).expect("source file"),
            b"mine\n"
        );
        assert_eq!(
            std::fs::read(to.join("theirs.txt")).expect("destination file"),
            b"theirs\n"
        );
    }

    /// A destination whose parent does not exist is neither "already there"
    /// nor a filesystem limitation, and must keep its own kind: the discard
    /// caller distinguishes those three answers.
    #[test]
    fn a_missing_destination_directory_keeps_its_own_error_kind() {
        let temp = tempfile::tempdir().expect("tempdir");
        let from = temp.path().join("from.txt");
        std::fs::write(&from, b"claimed\n").expect("source");

        let error = rename_no_replace(&from, &temp.path().join("gone").join("to.txt"))
            .expect_err("no such directory");

        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        assert!(from.exists());
    }
}
