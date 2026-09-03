// Path: crates/im_agent/src/server/shutdown/tests.rs
// Description: Unit tests for the drain gate: a held mutation keeps the drain waiting, and only idle reports drained

use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::logging::{LogConfig, LogLevel, Logger};
use crate::source_control::SourceControlLocks;

use super::{
    drain_source_control_bounded, finalize_shutdown, DrainOutcome, SHUTDOWN_EMERGENCY_BOUND,
};

/// Long enough that "the drain returned early" is unambiguous and short enough
/// that the test itself stays fast; the production bound is 450 s and has no
/// place in a test.
const TEST_BOUND: Duration = Duration::from_millis(400);

async fn test_logger() -> Logger {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock");
    Logger::init(LogConfig {
        log_dir: PathBuf::from(std::env::temp_dir())
            .join(format!("im_agent_shutdown_drain_{}", now.as_nanos())),
        min_level: LogLevel::Error,
        emit_stdio: false,
    })
    .await
    .expect("logger init")
}

/// The emergency bound is the whole point of the round-2 contract: it must be
/// longer than any bounded mutation the agent can still be running when the
/// request arrives (status 100 s + remote 180 s + status 100 s).
#[test]
fn the_emergency_bound_outlasts_the_longest_bounded_mutation() {
    assert!(SHUTDOWN_EMERGENCY_BOUND >= Duration::from_secs(380));
}

/// A mutation still holding its worktree lock keeps the drain waiting for the
/// whole budget. `drained: false` is only ever reached at the bound — never as
/// an early answer that lets the process exit over a running `git commit`.
#[tokio::test]
async fn a_held_mutation_keeps_the_drain_waiting_until_the_bound() {
    let logger = test_logger().await;
    let locks = SourceControlLocks::new();
    let root = std::env::temp_dir().join("im_agent_drain_gate_repo");
    locks.remember_git_dir(&root, &root);
    let guard = locks.acquire(&root).await.expect("lock");

    let started = Instant::now();
    let outcome = drain_source_control_bounded(&locks, &logger, "test", TEST_BOUND).await;
    let waited = started.elapsed();

    assert!(!outcome.drained, "a held mutation is not drained");
    assert_eq!(outcome.active_mutations, 1, "the residue is reported");
    assert!(
        waited >= TEST_BOUND,
        "the drain must spend its whole budget, waited {waited:?}"
    );
    drop(guard);
}

/// Once the mutation releases, the same drain reports the truth the other way:
/// drained, no residue, and back well inside the budget rather than sitting out
/// the rest of it.
#[tokio::test]
async fn a_released_mutation_lets_the_drain_finish_before_the_bound() {
    let logger = test_logger().await;
    let locks = SourceControlLocks::new();
    let root = std::env::temp_dir().join("im_agent_drain_release_repo");
    locks.remember_git_dir(&root, &root);
    let guard = locks.acquire(&root).await.expect("lock");
    tokio::spawn(async move {
        tokio::time::sleep(TEST_BOUND / 4).await;
        drop(guard);
    });

    let started = Instant::now();
    let outcome = drain_source_control_bounded(&locks, &logger, "test", TEST_BOUND).await;

    assert!(outcome.drained);
    assert_eq!(outcome.active_mutations, 0);
    assert!(started.elapsed() < TEST_BOUND, "returned as soon as it was idle");
}

/// Draining stops admission for good: the gate is set by the drain itself, not
/// by whoever called it, so a mutation arriving during the wait is refused
/// rather than extending the shutdown indefinitely.
#[tokio::test]
async fn the_drain_closes_admission_before_it_waits() {
    let logger = test_logger().await;
    let locks = SourceControlLocks::new();
    assert!(!locks.is_draining());

    let outcome = drain_source_control_bounded(&locks, &logger, "test", TEST_BOUND).await;

    assert!(outcome.drained, "an idle agent drains at once");
    assert!(locks.is_draining());
}

/// A drain that finished owns nothing: finalization must not reach for the
/// runner's forced-stop path at all. (The other direction — terminating the
/// trees a `drained: false` left behind — is deliberately not exercised here:
/// the kill is process-wide by design, so a test of it inside a parallel test
/// binary would reach the Git children of every other test running beside it.
/// The termination primitive itself is covered in `im_bundle`'s runner tests.)
#[tokio::test]
async fn finalizing_a_drained_shutdown_terminates_nothing() {
    let logger = test_logger().await;
    let terminated = finalize_shutdown(&logger, "test", DrainOutcome::idle()).await;
    assert_eq!(terminated, 0);
}
