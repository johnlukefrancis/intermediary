// Path: src-tauri/src/lib/macos_instance.rs
// Description: Mac application exclusivity before mutable startup and native activation

use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};
use objc2_foundation::NSString;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

pub struct Instance {
    _lock: File,
}

impl Instance {
    pub fn acquire(identifier: &str) -> Result<Option<Self>, String> {
        let directory = dirs::data_local_dir()
            .ok_or("Failed to resolve app local data for the Mac instance lock")?
            .join(identifier);
        std::fs::create_dir_all(&directory)
            .map_err(|err| format!("Failed to create Mac instance lock directory: {err}"))?;
        Self::lock(&directory.join("instance.lock"))
            .map_err(|err| format!("Failed to acquire Mac instance lock: {err}"))
    }

    fn lock(path: &Path) -> io::Result<Option<Self>> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(path)?;
        // Keep this inode in place: unlinking permits independent locks on different files.
        let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if result == 0 {
            return Ok(Some(Self { _lock: file }));
        }
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::WouldBlock {
            Ok(None)
        } else {
            Err(err)
        }
    }
}

pub fn activate_existing(identifier: &str) {
    autoreleasepool(|_| {
        let identifier = NSString::from_str(identifier);
        let current = NSRunningApplication::currentApplication();
        for app in NSRunningApplication::runningApplicationsWithBundleIdentifier(&identifier) {
            if app == current {
                continue;
            }
            app.unhide();
            if app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows) {
                return;
            }
        }
        eprintln!("Intermediary is already starting or running; this launch will exit.");
    });
}

#[cfg(test)]
mod tests {
    use super::Instance;

    #[test]
    fn instance_lock_excludes_duplicates_and_releases_without_deleting() {
        let directory = tempfile::tempdir().expect("test directory");
        let path = directory.path().join("instance.lock");
        std::fs::write(&path, b"persistent inode").expect("test lock file");
        let primary = Instance::lock(&path)
            .expect("primary lock")
            .expect("primary owner");
        assert!(Instance::lock(&path).expect("duplicate lock").is_none());
        drop(primary);
        assert_eq!(
            std::fs::read(&path).expect("retained lock file"),
            b"persistent inode"
        );
        assert!(Instance::lock(&path).expect("next launch lock").is_some());
    }

    #[test]
    fn invalid_lock_target_is_an_error_not_a_primary_instance() {
        let directory = tempfile::tempdir().expect("test directory");
        assert!(Instance::lock(directory.path()).is_err());
    }
}
