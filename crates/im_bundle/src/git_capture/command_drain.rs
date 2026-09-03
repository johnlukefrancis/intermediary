// Path: crates/im_bundle/src/git_capture/command_drain.rs
// Description: Bounded pipe drain for the Git runner: grace after exit, then termination of the whole process tree

use std::time::{Duration, Instant};

use super::command_child::{Collected, Streams};
use super::command_tree::GitProcessTree;

/// How long the streams get after the direct Git child exited by itself.
/// Its pipes normally close in the same instant; a hook or helper that
/// backgrounded a descendant holding stdout/stderr keeps them open, and the
/// caller (with the repo mutation lock in hand) must not wait on that.
const POST_EXIT_DRAIN_GRACE: Duration = Duration::from_secs(2);

/// How long a forced stop waits for the stream workers after the child and its
/// process tree were terminated. A descendant that escaped the tree (a `setsid`
/// on unix, a breakaway on Windows) can still hold a pipe; after this the
/// workers are detached and the stop result carries no output.
const FORCED_STOP_STREAM_WAIT: Duration = Duration::from_secs(2);

/// Drain after the direct child exited by itself. Bounded: on expiry the whole
/// process tree the child owned is terminated (its unix process group, its
/// Windows job object) and the readers are joined; either way the bytes already
/// collected and the child's exit status are what the caller gets.
///
/// Inside the grace nothing is terminated on either platform, so a helper that
/// closed the pipes and kept running on purpose — a credential-cache daemon —
/// outlives the command untouched.
pub(super) fn drain_after_exit(
    mut streams: Streams,
    tree: &GitProcessTree,
    child_pid: u32,
) -> Collected {
    if streams.wait_all(deadline(POST_EXIT_DRAIN_GRACE)) {
        return streams.into_collected();
    }
    let action = if tree.terminate() {
        if streams.wait_all(deadline(FORCED_STOP_STREAM_WAIT)) {
            "terminated its process tree"
        } else {
            "terminated its process tree; readers detached"
        }
    } else {
        // The tree is already empty, so the holder escaped it (a `setsid` on
        // unix, a breakaway on Windows). The detached readers end when that
        // descendant finally closes the pipe.
        "readers detached"
    };
    eprintln!(
        "im_bundle: git (pid {child_pid}) exited but a descendant held its output pipes for {}s: {action}",
        POST_EXIT_DRAIN_GRACE.as_secs()
    );
    streams.into_collected()
}

/// Drain after the runner stopped the child itself (timeout, cancellation, or
/// a failed wait): the tree was already terminated, so this is only the bounded
/// join before the readers are detached.
pub(super) fn drain_after_forced_stop(mut streams: Streams) -> Collected {
    streams.wait_all(deadline(FORCED_STOP_STREAM_WAIT));
    streams.into_collected()
}

fn deadline(wait: Duration) -> Instant {
    Instant::now() + wait
}
