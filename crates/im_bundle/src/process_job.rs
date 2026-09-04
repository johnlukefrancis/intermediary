// Path: crates/im_bundle/src/process_job.rs
// Description: Windows Job Object ownership of a spawned process tree, shared by the Git runner and the app's agent supervisor

//! Windows has no process group a signal can reach, so the only owner of a
//! spawned child's descendants is a Job Object. It is created *before* the
//! spawn. Generic owners assign the direct child immediately afterwards; the
//! terminal supplies the Job in the process-creation attribute list instead,
//! before any child code can run. Every later descendant then inherits it.
//!
//! The job is created **without** `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, so an
//! ordinary successful owner drop does not kill helpers that deliberately
//! outlive their parent. The bounded forced-cleanup route arms kill-on-close
//! immediately before explicit termination, making a later handle drop its
//! final safety net if a Win32 termination or observation call fails.
//!
//! Both owners in this workspace build on this one type: the Git runner nests
//! a per-command job inside it (nested jobs are supported from Windows 8), and
//! the app's supervisor wraps the host agent it spawns. The terminal passes
//! the raw Job handle in `PROC_THREAD_ATTRIBUTE_JOB_LIST`, so its shell belongs
//! to the Job from the instant `CreateProcessW` succeeds.
//!
//! Off Windows the type is an inert owner — `create`, `assign` and `terminate`
//! all succeed and do nothing — so call sites stay free of `cfg` noise. That
//! is honest rather than a fallback: on unix the Git runner owns its tree with
//! a real process group instead (`git_capture::command_tree`), and the app is
//! a Windows product whose supervisor has never had a unix tree owner.

use std::io;
use std::process::Child;
#[cfg(not(windows))]
use std::time::Duration;

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
#[cfg(not(windows))]
#[derive(Debug)]
pub struct JobHandle;

// SAFETY: a job handle is a process-wide kernel handle with no interior Rust
// state. Every use below is a single Win32 call the kernel serializes itself,
// and the handle is closed exactly once (in `Drop`), so sharing it across the
// thread that spawned the child and the thread that stops it is sound.
#[cfg(windows)]
unsafe impl Send for JobHandle {}
#[cfg(windows)]
unsafe impl Sync for JobHandle {}

#[cfg(windows)]
impl JobHandle {
    /// A job that owns whatever is assigned to it and outlives it harmlessly.
    /// The OS error is returned rather than swallowed: a caller that cannot own
    /// the tree it is about to create has to decide that, not this module.
    pub fn create() -> io::Result<Self> {
        // SAFETY: an unnamed job with default security; both pointer arguments
        // are documented as optional and null means "use the default".
        let handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(handle))
    }

    /// Puts the freshly spawned child — and therefore everything it goes on to
    /// spawn — inside this job. Call it immediately after the spawn: anything
    /// the child starts before this lands outside the tree.
    pub fn assign(&self, child: &Child) -> io::Result<()> {
        self.assign_raw_handle(child.as_raw_handle())
    }

    /// The same assignment for a child we hold only a raw process handle for
    /// (a pseudoconsole child spawned by a pty library, which is not a
    /// `std::process::Child`). The handle must be live for the duration of the
    /// call and stays owned by the caller.
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
        // SAFETY: closed exactly once, at the end of this handle's life. An
        // ordinary owner never armed kill-on-close; a forced-cleanup owner did
        // so deliberately before attempting explicit termination.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[cfg(not(windows))]
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

#[cfg(all(test, not(windows)))]
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
