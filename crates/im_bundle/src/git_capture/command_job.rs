// Path: crates/im_bundle/src/git_capture/command_job.rs
// Description: Windows Job Object primitives: create the job, assign a spawned child, terminate the tree on demand

//! Windows has no process group a signal can reach, so the owner of a Git
//! child's descendants (hooks, `ssh`, credential helpers, `git-remote-*`) is a
//! Job Object instead. It is created before the spawn and the direct child is
//! assigned to it immediately afterwards — every process that child starts from
//! then on is created inside the same job.
//!
//! The job is deliberately created *without*
//! `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, so it is a handle on the tree and not
//! the tree's lifetime: closing it kills nothing, exactly as dropping a unix
//! process group id kills nothing. Every death is an explicit
//! [`JobHandle::terminate`], at the three moments the runner decides the tree
//! must not outlive the call — a forced stop (timeout or cancellation), an
//! expired post-exit pipe drain, and the agent's shutdown finalization. A
//! helper that deliberately outlives Git after closing its pipes (a
//! credential-cache daemon) therefore survives here exactly as it does on unix.

use std::os::windows::io::AsRawHandle;
use std::process::Child;
use std::ptr;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, TerminateJobObject,
};

/// An owned, unnamed job object with no limits set: it ends nothing by itself,
/// and `terminate` is the only thing that kills what is inside it. Closed
/// exactly once, in `Drop`.
pub(super) struct JobHandle(HANDLE);

// SAFETY: a job handle is a process-wide kernel handle with no interior Rust
// state. Every use below is a single Win32 call the kernel serializes itself,
// and the handle is closed exactly once (in `Drop`), so sharing it across the
// runner's thread and the shutdown owner's thread is sound.
unsafe impl Send for JobHandle {}
unsafe impl Sync for JobHandle {}

impl JobHandle {
    /// A job that owns whatever is assigned to it and outlives it harmlessly.
    /// `None` when the create call fails: the caller then runs without a tree
    /// owner, exactly as it did before job objects existed, rather than
    /// refusing to run Git.
    pub(super) fn create() -> Option<Self> {
        // SAFETY: an unnamed job with default security; both pointer arguments
        // are documented as optional and null means "use the default".
        let handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if handle.is_null() {
            return None;
        }
        Some(Self(handle))
    }

    /// Puts the freshly spawned child — and therefore everything it goes on to
    /// spawn — inside this job.
    pub(super) fn assign(&self, child: &Child) -> bool {
        let process = child.as_raw_handle() as HANDLE;
        // SAFETY: both handles are live: the job is owned by `self`, and the
        // process handle is owned by `child`, which outlives this call.
        unsafe { AssignProcessToJobObject(self.0, process) != 0 }
    }

    /// Kills every process still in the job. Safe to call more than once, and
    /// the only thing that ends this tree.
    pub(super) fn terminate(&self) -> bool {
        // SAFETY: `self.0` is a live job handle for as long as `self` exists.
        unsafe { TerminateJobObject(self.0, 1) != 0 }
    }
}

impl Drop for JobHandle {
    fn drop(&mut self) {
        // SAFETY: closed exactly once, at the end of this handle's life. With
        // no kill-on-close limit this only releases the runner's grip on the
        // tree: anything still inside the job keeps running unless
        // `terminate` was called first.
        unsafe {
            CloseHandle(self.0);
        }
    }
}
