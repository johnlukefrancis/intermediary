// Path: crates/im_bundle/src/git_capture/command_stop.rs
// Description: Forced stop of a running Git child: ask the process tree to end, then kill it and reap the child

use std::process::Child;
use std::thread;
use std::time::{Duration, Instant};

use super::command::KillPolicy;
use super::command_tree::GitProcessTree;

const POLL_INTERVAL: Duration = Duration::from_millis(10);
const GRACEFUL_TERMINATION_WAIT: Duration = Duration::from_secs(5);

/// Stops the child and everything else in its tree, then reaps the child.
/// `Graceful` first asks the tree to terminate so Git's lockfile cleanup runs;
/// after that wait, or immediately for `Immediate`, the whole tree is killed.
/// The tree — not `Child::kill` — is what a stop targets: a hook, `ssh`, or a
/// credential helper that outlives Git still holds the runner's output pipes.
///
/// This is the first of the three places that terminate a tree (the others are
/// the expired post-exit drain and shutdown finalization); nothing dies here
/// unless the caller asked for a stop.
pub(super) fn stop_child(child: &mut Child, kill_policy: KillPolicy, tree: &GitProcessTree) {
    if kill_policy == KillPolicy::Graceful && tree.request_termination() {
        let deadline = Instant::now() + GRACEFUL_TERMINATION_WAIT;
        while Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(_)) => {
                    // Git exited on its own; whatever it left in the tree must
                    // not keep the output pipes open.
                    tree.terminate();
                    return;
                }
                Ok(None) => thread::sleep(POLL_INTERVAL),
                Err(_) => break,
            }
        }
    }
    tree.terminate();
    let _ = child.kill();
    let _ = child.wait();
}
