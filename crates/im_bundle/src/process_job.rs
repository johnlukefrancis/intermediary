// Path: crates/im_bundle/src/process_job.rs
// Description: Windows Job Object ownership of a spawned process tree, shared by the Git runner and the app's agent supervisor

//! Windows Job ownership; macOS uses an isolated process session established before spawn.
//! Forced cleanup is explicit; ordinary owner drop does not terminate surviving helpers.

#[cfg(not(target_os = "macos"))]
use std::io;
#[cfg(not(target_os = "macos"))]
use std::process::Child;
#[cfg(not(any(windows, target_os = "macos")))]
use std::time::Duration;

#[cfg(target_os = "macos")]
pub use crate::macos_process_session::ProcessSession as JobHandle;

#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, RawHandle};
#[cfg(windows)]
use std::ptr;

#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(windows)]
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, TerminateJobObject,
};

/// An owned, unnamed job object with no initial limits. Forced cleanup may arm
/// kill-on-close; the handle is still closed exactly once, in `Drop`.
#[cfg(windows)]
#[derive(Debug)]
pub struct JobHandle(HANDLE);

/// The inert owner used off Windows: it owns nothing and kills nothing, so
/// callers keep one code path on every platform.
#[cfg(not(any(windows, target_os = "macos")))]
#[derive(Debug)]
pub struct JobHandle;

// SAFETY: the kernel serializes use of this process-wide handle, closed once by Drop.
#[cfg(windows)]
unsafe impl Send for JobHandle {}
#[cfg(windows)]
unsafe impl Sync for JobHandle {}

#[cfg(windows)]
impl JobHandle {
    /// Creates an unlimited Job; forced cleanup explicitly arms kill-on-close.
    pub fn create() -> io::Result<Self> {
        // SAFETY: an unnamed job with default security; both pointer arguments
        // are documented as optional and null means "use the default".
        let handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(handle))
    }

    /// Assign immediately after spawn; children created before assignment are outside this Job.
    pub fn assign(&self, child: &Child) -> io::Result<()> {
        self.assign_raw_handle(child.as_raw_handle())
    }

    /// The borrowed process handle must remain live for this call; ownership stays with the caller.
    pub fn assign_raw_handle(&self, process: RawHandle) -> io::Result<()> {
        // SAFETY: both handles are live: the job is owned by `self`, and the
        // caller guarantees the process handle outlives this call.
        if unsafe { AssignProcessToJobObject(self.0, process as HANDLE) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Borrows the Job handle for a `CreateProcessW` attribute list. The
    /// returned handle remains owned by `self` and must not be closed.
    pub fn raw_handle(&self) -> RawHandle {
        self.0 as RawHandle
    }

    /// Kills every process still in the job. Safe to call more than once, and
    /// the only thing that ends this tree.
    pub fn terminate(&self) -> io::Result<()> {
        // SAFETY: `self.0` is a live job handle for as long as `self` exists.
        if unsafe { TerminateJobObject(self.0, 1) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

#[cfg(windows)]
impl Drop for JobHandle {
    fn drop(&mut self) {
        // SAFETY: this owner closes its live handle exactly once.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
impl JobHandle {
    /// Always succeeds: there is nothing to create.
    pub fn create() -> io::Result<Self> {
        Ok(Self)
    }

    /// Always succeeds and claims nothing.
    pub fn assign(&self, _child: &Child) -> io::Result<()> {
        Ok(())
    }

    /// Always succeeds and claims nothing; the raw-handle form exists so a pty
    /// child's spawn site stays free of `cfg` noise too.
    pub fn assign_raw_handle(&self, _process: usize) -> io::Result<()> {
        Ok(())
    }

    /// Always succeeds and kills nothing: off Windows the caller's own kill of
    /// the direct child is the whole story.
    pub fn terminate(&self) -> io::Result<()> {
        Ok(())
    }

    pub fn terminate_and_observe(&self, _timeout: Duration) -> io::Result<()> {
        Ok(())
    }

    pub fn active_processes(&self) -> io::Result<u32> {
        Ok(0)
    }
}

#[cfg(all(test, not(any(windows, target_os = "macos"))))]
mod tests {
    use super::JobHandle;
    use std::process::{Command, Stdio};

    /// The inert owner is a complete owner off Windows: nothing a call site
    /// does with it can fail, so no call site needs a `cfg` around it.
    #[test]
    fn the_inert_owner_accepts_the_whole_contract() {
        let job = JobHandle::create().expect("create");
        let mut child = Command::new("sh")
            .arg("-c")
            .arg("exit 0")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        job.assign(&child).expect("assign");
        job.terminate().expect("terminate");
        // Terminating owns nothing off Windows: the child is still ours to reap.
        let status = child.wait().expect("wait");
        assert!(status.success());
    }
}
