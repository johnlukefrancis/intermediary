// Path: crates/im_bundle/src/git_capture/command_tree.rs
// Description: The process tree one Git child owns (unix process group, Windows job object) and the live-tree registry

//! One Git command owns a whole process tree, not just the process this crate
//! spawned: hooks, `ssh`, credential helpers and `git-remote-*` all inherit its
//! output pipes and can outlive it. This module is that tree's only owner —
//! created before the spawn, attached to the child immediately after it, and
//! terminated by a timeout, a cancellation, a post-exit pipe holder, or a
//! shutdown deadline; never a bare `Child::kill`, which reaches the direct
//! child alone. Termination is always explicit, so a helper that closed Git's
//! pipes and kept running is left alone.
//!
//! A mutation ([`KillPolicy::Graceful`]) never runs without that owner: if the
//! tree cannot be created or the child cannot be joined to it, the command is
//! refused rather than silently downgraded to a `Child::kill` that cannot reach
//! a hook holding `.git/index.lock`. A read runs on unowned and says so once.
//! Which of those two a failure earns, and how it is worded, belongs to
//! `command_tree_owner`; this module owns the tree itself.
//!
//! Every live tree is also registered process-wide, so the one caller that owns
//! finality rather than one command — an agent that has reached its shutdown
//! emergency bound — can terminate what is left through
//! [`terminate_git_process_trees`] instead of exiting over the top of it.

use std::collections::BTreeMap;
use std::io;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

#[cfg(unix)]
use std::sync::atomic::AtomicU32;

#[cfg(windows)]
use crate::process_job::JobHandle;

use super::command::KillPolicy;
use super::command_failure::GitCommandFailure;
use super::command_tree_owner::{log_unowned_read_once, no_owner_failure};

static NEXT_TREE_ID: AtomicU64 = AtomicU64::new(0);
static LIVE_TREES: Mutex<BTreeMap<u64, Arc<TreeOwner>>> = Mutex::new(BTreeMap::new());

/// The tree owner for one Git command, registered for as long as that command
/// runs. Dropping it unregisters the tree and terminates nothing: on Windows
/// the job handle is closed without a kill-on-close limit, exactly as a unix
/// process group id is simply forgotten.
pub(super) struct GitProcessTree {
    id: u64,
    owner: Arc<TreeOwner>,
    registered: bool,
}

impl GitProcessTree {
    pub(super) fn from_owner(owner: TreeOwner) -> Self {
        Self {
            id: NEXT_TREE_ID.fetch_add(1, Ordering::Relaxed),
            owner: Arc::new(owner),
            registered: false,
        }
    }

    /// Applies whatever the spawn itself has to carry for the child to land in
    /// this tree (a new process group on unix; nothing on Windows, where the
    /// job is joined after the spawn instead).
    pub(super) fn prepare(&self, command: &mut Command) {
        self.owner.prepare(command);
    }

    /// Claims the freshly spawned child. A mutation whose child cannot be
    /// joined gets the same refusal the pre-spawn path returns, and its caller
    /// stops the child; a read is logged once and runs on unowned. Until this
    /// succeeds the tree owns nothing, so a `terminate` before it is `false`.
    pub(super) fn attach(
        &mut self,
        child: &Child,
        kill_policy: KillPolicy,
    ) -> Result<(), GitCommandFailure> {
        match self.owner.attach(child) {
            Ok(()) => {
                if self.owner.attached.load(Ordering::SeqCst) {
                    live_trees().insert(self.id, Arc::clone(&self.owner));
                    self.registered = true;
                }
                Ok(())
            }
            Err(error) if kill_policy == KillPolicy::Graceful => Err(no_owner_failure(&error, true)),
            Err(error) => {
                log_unowned_read_once(&error);
                Ok(())
            }
        }
    }

    /// Kills the whole tree — the only thing that ends it, called on a forced
    /// stop, on post-exit drain expiry, and at shutdown finalization. `true`
    /// when it reached a live tree.
    pub(super) fn terminate(&self) -> bool {
        self.owner.terminate()
    }

    /// Asks the tree to end on its own so Git's lockfile cleanup runs.
    /// `false` on platforms with no such signal, which is the caller's cue to
    /// go straight to `terminate`.
    pub(super) fn request_termination(&self) -> bool {
        self.owner.request_termination()
    }
}

impl Drop for GitProcessTree {
    fn drop(&mut self) {
        if self.registered {
            live_trees().remove(&self.id);
        }
    }
}

/// Kills every Git process tree this process still owns and reports how many
/// were reached. The caller is an agent whose shutdown drain hit its emergency
/// bound: the mutations behind these trees are already `unknown`, and leaving
/// the tree running past the process that owns it is what the review called
/// out, so this is the last act before exit — never an ordinary cancellation
/// path, which goes through the command's own timeout.
pub fn terminate_git_process_trees() -> usize {
    let owners: Vec<Arc<TreeOwner>> = live_trees().values().map(Arc::clone).collect();
    owners.iter().filter(|owner| owner.terminate()).count()
}

fn live_trees() -> MutexGuard<'static, BTreeMap<u64, Arc<TreeOwner>>> {
    match LIVE_TREES.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// The platform half: a process group id on unix, a job object on Windows.
/// `attached` is the shared half — nothing may be signalled before the child is
/// actually in the tree.
pub(super) struct TreeOwner {
    attached: AtomicBool,
    #[cfg(unix)]
    pgid: AtomicU32,
    #[cfg(windows)]
    job: Option<JobHandle>,
}

#[cfg(unix)]
impl TreeOwner {
    /// Infallible: a process group costs nothing until the spawn asks for it,
    /// so unix has no unowned case and `unowned` is simply the same owner.
    pub(super) fn new() -> io::Result<Self> {
        Ok(Self::unowned())
    }

    pub(super) fn unowned() -> Self {
        Self {
            attached: AtomicBool::new(false),
            pgid: AtomicU32::new(0),
        }
    }

    /// A new process group, so one signal reaches every grandchild that
    /// inherited the runner's output pipes.
    fn prepare(&self, command: &mut Command) {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    /// The group id is the child's own pid, which only exists after the spawn.
    fn attach(&self, child: &Child) -> io::Result<()> {
        self.pgid.store(child.id(), Ordering::SeqCst);
        self.attached.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn terminate(&self) -> bool {
        self.signal(libc::SIGKILL)
    }

    fn request_termination(&self) -> bool {
        self.signal(libc::SIGTERM)
    }

    fn signal(&self, signal: libc::c_int) -> bool {
        if !self.attached.load(Ordering::SeqCst) {
            return false;
        }
        let Ok(pid) = libc::pid_t::try_from(self.pgid.load(Ordering::SeqCst)) else {
            return false;
        };
        // SAFETY: the group id is the pid of a child spawned with
        // `process_group(0)`. The kernel keeps that number reserved while any
        // member of the group is alive, which is exactly the case this signal
        // exists for; an empty group answers ESRCH instead.
        unsafe { libc::kill(-pid, signal) == 0 }
    }
}

#[cfg(windows)]
impl TreeOwner {
    pub(super) fn new() -> io::Result<Self> {
        Ok(Self {
            attached: AtomicBool::new(false),
            job: Some(JobHandle::create()?),
        })
    }

    /// A read that could not get a job object: it owns nothing, so it claims
    /// nothing and terminates nothing.
    pub(super) fn unowned() -> Self {
        Self {
            attached: AtomicBool::new(false),
            job: None,
        }
    }

    /// Nothing: the child joins the job right after it is spawned, because a
    /// job cannot be inherited through `CreateProcess` flags the way a process
    /// group is.
    fn prepare(&self, _command: &mut Command) {}

    fn attach(&self, child: &Child) -> io::Result<()> {
        let Some(job) = self.job.as_ref() else {
            return Ok(());
        };
        job.assign(child)?;
        self.attached.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// The job carries no kill-on-close limit, so this call — never the
    /// handle's `Drop` — is what ends the tree.
    fn terminate(&self) -> bool {
        if !self.attached.load(Ordering::SeqCst) {
            return false;
        }
        self.job
            .as_ref()
            .is_some_and(|job| job.terminate().is_ok())
    }

    /// Windows has no cooperative termination signal for a console child, so
    /// the caller's timeout is the only bound and terminating the job is final.
    fn request_termination(&self) -> bool {
        false
    }
}
