// Path: src-tauri/src/lib/agent/wsl_agent_termination_tests.rs
// Description: Tests for the WSL emergency stop's drain envelope and process-tree escalation

use super::{
    terminate_matching_wsl_agent_processes, WslTerminateBudget, WslTerminateOutcome,
    WslTerminationChannel,
};
use std::time::Duration;

/// A real 480 s envelope has no place in a test, so the decision logic is what a
/// short one exercises: the same branches, the same order, none of the wait.
const TEST_BUDGET: WslTerminateBudget = WslTerminateBudget {
    drain_wait: Duration::from_millis(150),
    kill_grace: Duration::from_millis(150),
};

/// A scripted distro. `pids` is answered one entry per probe (the last entry
/// repeats), so a test states exactly what the agent does over time; a
/// `kill_process_trees` call switches to `after_kill` from then on.
struct ScriptedChannel {
    probes: Vec<Result<Vec<u32>, String>>,
    probe_index: usize,
    after_kill: Option<Vec<u32>>,
    killed: bool,
    term_calls: usize,
    kill_calls: usize,
    kill_targets: Vec<u32>,
    signalled: usize,
}

impl ScriptedChannel {
    fn new(probes: Vec<Result<Vec<u32>, String>>) -> Self {
        Self {
            probes,
            probe_index: 0,
            after_kill: None,
            killed: false,
            term_calls: 0,
            kill_calls: 0,
            kill_targets: Vec::new(),
            signalled: 0,
        }
    }

    fn vanishing_after_kill(mut self, signalled: usize) -> Self {
        self.after_kill = Some(Vec::new());
        self.signalled = signalled;
        self
    }
}

impl WslTerminationChannel for ScriptedChannel {
    fn list_pids(&mut self) -> Result<Vec<u32>, String> {
        if self.killed {
            if let Some(after) = self.after_kill.as_ref() {
                return Ok(after.clone());
            }
        }
        let index = self.probe_index.min(self.probes.len().saturating_sub(1));
        self.probe_index += 1;
        self.probes[index].clone()
    }

    fn send_term(&mut self, _pids: &[u32]) -> Result<Option<String>, String> {
        self.term_calls += 1;
        Ok(None)
    }

    fn kill_process_trees(&mut self, pids: &[u32]) -> Result<usize, String> {
        self.kill_calls += 1;
        self.kill_targets = pids.to_vec();
        self.killed = true;
        Ok(self.signalled)
    }
}

/// The ordinary route: the host agent already forwarded `shutdown` and the WSL
/// agent is gone. Nothing is signalled, so stop and restart stay fast.
#[test]
fn an_agent_that_already_exited_is_never_signalled() {
    let mut channel = ScriptedChannel::new(vec![Ok(vec![])]);
    let outcome =
        terminate_matching_wsl_agent_processes(&mut channel, TEST_BUDGET, "port-listener :3142")
            .expect("no match is not an error");

    assert_eq!(outcome, WslTerminateOutcome::NoMatch);
    assert_eq!(channel.term_calls, 0);
    assert_eq!(channel.kill_calls, 0);
}

/// The agent drained inside the envelope, so its own shutdown owned the Git
/// process trees. Killing anything here would be the defect this route exists to
/// avoid, so the sweep must not run.
#[test]
fn an_agent_that_drains_inside_the_envelope_is_never_swept() {
    let mut channel = ScriptedChannel::new(vec![Ok(vec![4242]), Ok(vec![4242]), Ok(vec![])]);
    let outcome =
        terminate_matching_wsl_agent_processes(&mut channel, TEST_BUDGET, "port-listener :3142")
            .expect("a drained agent is not an error");

    assert_eq!(outcome, WslTerminateOutcome::TerminatedWithTerm);
    assert_eq!(channel.term_calls, 1);
    assert_eq!(channel.kill_calls, 0, "the tree sweep must not run");
}

/// The envelope expired with the agent still there. The sweep runs exactly once,
/// against the pids that actually survived, and the count it reports travels out
/// with the outcome.
#[test]
fn an_agent_that_outlives_the_envelope_gets_one_tree_sweep() {
    let mut channel = ScriptedChannel::new(vec![Ok(vec![4242])]).vanishing_after_kill(3);
    let outcome =
        terminate_matching_wsl_agent_processes(&mut channel, TEST_BUDGET, "port-listener :3142")
            .expect("the sweep ended the agent");

    assert_eq!(
        outcome,
        WslTerminateOutcome::TerminatedWithKill { signalled: 3 }
    );
    assert_eq!(channel.term_calls, 1, "TERM comes before the envelope");
    assert_eq!(channel.kill_calls, 1, "exactly one sweep");
    assert_eq!(channel.kill_targets, vec![4242]);
}

/// A sweep that did not end the agent is a failure with the count in it, not a
/// silent success.
#[test]
fn an_agent_that_survives_the_sweep_is_an_error() {
    let mut channel = ScriptedChannel::new(vec![Ok(vec![4242])]);
    let error =
        terminate_matching_wsl_agent_processes(&mut channel, TEST_BUDGET, "port-listener :3142")
            .expect_err("a surviving agent must not be reported as stopped");

    assert!(error.contains("did not exit after TERM"), "{error}");
    assert!(error.contains("process-tree KILL"), "{error}");
    assert_eq!(channel.kill_calls, 1);
}

/// A `wsl.exe` round trip that could not answer is not proof the agent is gone.
/// Over an envelope made of hundreds of them, one hiccup must not end the wait
/// and skip the escalation behind it.
#[test]
fn a_probe_failure_inside_the_envelope_never_ends_the_wait() {
    let mut channel = ScriptedChannel::new(vec![
        Ok(vec![4242]),
        Err("WSL command timed out after 5000ms (distro=Ubuntu)".to_string()),
        Ok(vec![4242]),
    ])
    .vanishing_after_kill(2);
    let outcome =
        terminate_matching_wsl_agent_processes(&mut channel, TEST_BUDGET, "port-listener :3142")
            .expect("a transient probe failure must not fail the stop");

    assert_eq!(
        outcome,
        WslTerminateOutcome::TerminatedWithKill { signalled: 2 }
    );
    assert_eq!(channel.kill_calls, 1);
}

/// The re-read after the envelope is the authoritative one: a channel that
/// cannot answer *there* fails the stop rather than sweeping blind.
#[test]
fn a_probe_failure_at_the_authoritative_re_read_fails_the_stop() {
    let mut channel = ScriptedChannel::new(vec![
        Ok(vec![4242]),
        Err("WSL command timed out after 5000ms (distro=Ubuntu)".to_string()),
    ]);
    let error =
        terminate_matching_wsl_agent_processes(&mut channel, TEST_BUDGET, "port-listener :3142")
            .expect_err("an unreadable distro must not escalate blind");

    assert!(error.contains("timed out"), "{error}");
    assert_eq!(
        channel.kill_calls, 0,
        "nothing is swept on an unknown state"
    );
}

/// An agent that exits in the same instant the envelope expires is drained, not
/// killed: the authoritative re-read is what decides, never the clock.
#[test]
fn an_agent_that_exits_at_the_envelope_edge_is_reported_as_drained() {
    let mut channel = ScriptedChannel::new(vec![Ok(vec![4242]), Ok(vec![4242]), Ok(vec![])]);
    // The envelope is shorter than the probe that finds it gone, so the loop
    // gives up first and the re-read is what sees the exit.
    let budget = WslTerminateBudget {
        drain_wait: Duration::from_millis(0),
        kill_grace: Duration::from_millis(50),
    };
    let outcome =
        terminate_matching_wsl_agent_processes(&mut channel, budget, "port-listener :3142")
            .expect("an exit at the edge is not an error");

    assert_eq!(outcome, WslTerminateOutcome::TerminatedWithTerm);
    assert_eq!(channel.kill_calls, 0);
}
