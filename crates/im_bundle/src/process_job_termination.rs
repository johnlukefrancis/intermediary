// Path: crates/im_bundle/src/process_job_termination.rs
// Description: Bounded forced termination and observation for a Windows Job Object

use super::process_job::JobHandle;
use std::io;
use std::thread;
use std::time::{Duration, Instant};
use windows_sys::Win32::System::JobObjects::{
    JobObjectBasicAccountingInformation, JobObjectExtendedLimitInformation,
    QueryInformationJobObject, SetInformationJobObject, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};

const JOB_EMPTY_POLL: Duration = Duration::from_millis(10);

impl JobHandle {
    /// Arms kill-on-close, terminates the tree, and observes Job emptiness
    /// within `timeout`. Ordinary successful exits must not call this route:
    /// only forced cleanup owns detached descendants. Arming first also makes
    /// dropping the handle a final safety net when a later Win32 call fails.
    pub fn terminate_and_observe(&self, timeout: Duration) -> io::Result<()> {
        let arm_error = self.arm_kill_on_close().err();
        if let Err(terminate_error) = self.terminate() {
            return Err(combined_job_error(
                "Failed to terminate Job Object",
                terminate_error,
                arm_error,
            ));
        }
        let deadline = Instant::now() + timeout;
        loop {
            match self.active_processes() {
                Ok(0) => return Ok(()),
                Ok(active) if Instant::now() >= deadline => {
                    let suffix = arm_error
                        .map(|error| format!("; kill-on-close also failed: {error}"))
                        .unwrap_or_default();
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!(
                            "Job Object still reports {active} active process(es) after {}ms{suffix}",
                            timeout.as_millis()
                        ),
                    ));
                }
                Ok(_) => thread::sleep(
                    JOB_EMPTY_POLL.min(deadline.saturating_duration_since(Instant::now())),
                ),
                Err(query_error) => {
                    return Err(combined_job_error(
                        "Failed to observe Job Object termination",
                        query_error,
                        arm_error,
                    ));
                }
            }
        }
    }

    pub fn active_processes(&self) -> io::Result<u32> {
        let mut accounting = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        // SAFETY: the Job handle is live; the output buffer and its exact byte
        // count remain valid for the call.
        if unsafe {
            QueryInformationJobObject(
                self.raw_handle() as _,
                JobObjectBasicAccountingInformation,
                (&mut accounting as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
                std::mem::size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(accounting.ActiveProcesses)
    }

    fn arm_kill_on_close(&self) -> io::Result<()> {
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: the Job handle and exact extended-limit buffer are live for
        // this call. The Job was created with no other limits.
        if unsafe {
            SetInformationJobObject(
                self.raw_handle() as _,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

fn combined_job_error(
    context: &str,
    primary: io::Error,
    arm_error: Option<io::Error>,
) -> io::Error {
    let suffix = arm_error
        .map(|error| format!("; kill-on-close also failed: {error}"))
        .unwrap_or_default();
    io::Error::new(primary.kind(), format!("{context}: {primary}{suffix}"))
}
