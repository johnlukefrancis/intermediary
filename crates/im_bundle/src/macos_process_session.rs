// Path: crates/im_bundle/src/macos_process_session.rs
// Description: macOS process-session ownership for native terminals and supervised agents

use std::io;
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct ProcessSession {
    session: AtomicI32,
}

impl ProcessSession {
    pub fn create() -> io::Result<Self> {
        Ok(Self {
            session: AtomicI32::new(0),
        })
    }

    pub fn bind(&self, pid: u32) -> io::Result<()> {
        let pid = i32::try_from(pid).map_err(|_| io::Error::other("Invalid PTY process id"))?;
        if pid <= 0 {
            return Err(io::Error::other(
                "A process session needs a positive leader PID",
            ));
        }
        self.session
            .compare_exchange(0, pid, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| io::Error::other("PTY session already owned"))?;
        Ok(())
    }

    pub fn hangup(&self) -> io::Result<()> {
        self.signal(libc::SIGHUP)
    }

    pub fn assign(&self, child: &std::process::Child) -> io::Result<()> {
        // SAFETY: getsid queries the spawned child without modifying its process state.
        if unsafe { libc::getsid(child.id() as i32) } != child.id() as i32 {
            return Err(io::Error::other(
                "The spawned agent has no isolated process session",
            ));
        }
        self.bind(child.id())
    }

    pub fn terminate(&self) -> io::Result<()> {
        self.terminate_and_observe(Duration::from_millis(500))
    }

    pub fn terminate_and_observe(&self, timeout: Duration) -> io::Result<()> {
        let deadline = Instant::now() + timeout;
        loop {
            self.signal(libc::SIGKILL)?;
            if self.members()?.is_empty() {
                self.session.store(0, Ordering::Release);
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "PTY session still has live processes",
                ));
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn signal(&self, signal: i32) -> io::Result<()> {
        for pid in self.members()? {
            // SAFETY: only members of the session established by portable-pty's setsid are targeted.
            if unsafe { libc::getsid(pid) } != self.session.load(Ordering::Acquire) {
                continue;
            }
            // SAFETY: a positive PID addresses exactly one process, never our caller's group.
            if unsafe { libc::kill(pid, signal) } != 0 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() != Some(libc::ESRCH) {
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    fn members(&self) -> io::Result<Vec<i32>> {
        let session = self.session.load(Ordering::Acquire);
        if session <= 0 {
            return Ok(Vec::new());
        }
        // SAFETY: a null buffer queries the process count without reading memory.
        let count = unsafe { libc::proc_listallpids(std::ptr::null_mut(), 0) };
        if count <= 0 {
            return Err(io::Error::last_os_error());
        }
        let mut pids = vec![0i32; count as usize + 1024];
        let bytes = i32::try_from(pids.len() * std::mem::size_of::<i32>())
            .map_err(|_| io::Error::other("Process list exceeds native buffer size"))?;
        // SAFETY: the buffer has bytes writable bytes and holds native PID values.
        let count = unsafe { libc::proc_listallpids(pids.as_mut_ptr().cast(), bytes) };
        if count <= 0 || count as usize >= pids.len() {
            return Err(io::Error::other(
                "Could not capture the complete PTY process list",
            ));
        }
        pids.truncate(count as usize);
        pids.retain(|&pid| {
            // SAFETY: getsid only queries a positive process ID.
            if pid <= 0 || unsafe { libc::getsid(pid) } != session {
                return false;
            }
            let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::uninit();
            let size = std::mem::size_of::<libc::proc_bsdinfo>() as i32;
            // SAFETY: info has the exact layout/size required by PROC_PIDTBSDINFO.
            let read = unsafe {
                libc::proc_pidinfo(
                    pid,
                    libc::PROC_PIDTBSDINFO,
                    0,
                    info.as_mut_ptr().cast(),
                    size,
                )
            };
            // SAFETY: a full read initialized info; zombies have no executing resources.
            read != size || unsafe { info.assume_init().pbi_status } != 5
        });
        Ok(pids)
    }
}
