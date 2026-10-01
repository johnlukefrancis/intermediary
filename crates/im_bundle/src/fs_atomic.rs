// Path: crates/im_bundle/src/fs_atomic.rs
// Description: Atomic no-replace rename for Windows, Linux and macOS

use std::io::{Error, ErrorKind, Result};
use std::path::Path;

/// Atomically moves onto an empty destination; an occupied destination preserves both paths.
/// Unsupported filesystems return Unsupported and must never fall back to a replacing rename.
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

/// Unsupported no-replace flags remain distinct from an occupied destination.
#[cfg(any(target_os = "linux", target_os = "macos"))]
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

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn interior_nul(error: std::ffi::NulError) -> Error {
    Error::new(
        ErrorKind::InvalidInput,
        format!("path contains a NUL byte: {error}"),
    )
}

#[cfg(windows)]
pub fn rename_no_replace(from: &Path, to: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;

    let from_w: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to_w: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: both NUL-terminated buffers outlive this call; zero flags exclude replacement.
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
        Some(code) if code as u32 == ERROR_ALREADY_EXISTS || code as u32 == ERROR_FILE_EXISTS => {
            Error::new(
                ErrorKind::AlreadyExists,
                format!("the destination already exists: {error}"),
            )
        }
        _ => error,
    }
}

#[cfg(target_os = "macos")]
pub fn rename_no_replace(from: &Path, to: &Path) -> Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let from_c = CString::new(from.as_os_str().as_bytes()).map_err(interior_nul)?;
    let to_c = CString::new(to.as_os_str().as_bytes()).map_err(interior_nul)?;
    // SAFETY: both strings outlive the call; RENAME_EXCL atomically refuses an occupied target.
    let outcome = unsafe { libc::renamex_np(from_c.as_ptr(), to_c.as_ptr(), libc::RENAME_EXCL) };
    if outcome == 0 {
        Ok(())
    } else {
        Err(classify(Error::last_os_error()))
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
compile_error!("rename_no_replace requires Windows, Linux or macOS");

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

    /// An occupied target preserves both independent copies.
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

    /// A directory and its contents move as one atomic operation.
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

    /// Occupied directory names preserve both trees.
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

    /// Missing parents remain distinguishable from occupied targets and unsupported filesystems.
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
