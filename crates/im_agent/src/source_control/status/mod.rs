// Path: crates/im_agent/src/source_control/status/mod.rs
// Description: Capture `git status --porcelain=v2` for one repo root and project it onto the wire shape

mod index_tree;
mod project;
mod snapshot;
pub(in crate::source_control) mod stamp;
#[cfg(test)]
mod tests_projection;

use std::path::Path;

use im_bundle::git::{BundleCancelToken, GitCommandOutput};

use crate::error::AgentError;
use crate::protocol::SourceControlStatus;

use self::index_tree::capture_index_tree_sha;
use self::project::project_status;
use self::snapshot::capture_snapshot_id;
use self::stamp::stamp_worktree_entries;
use crate::source_control::discard::quarantine::sweep_stale_quarantine;
use crate::source_control::locks::SourceControlLocks;
use crate::source_control::runner::{self, GitCall, READ_TIMEOUT, STATUS_LIMIT};

/// How many times a torn index/porcelain pair is retried before the identity
/// is given up on. Chosen to absorb one ordinary concurrent `git add` without
/// looping noticeably; a repository torn on every one of three consecutive
/// reads is under sustained enough concurrent mutation that no snapshot would
/// be stable regardless.
const TORN_STATUS_RETRIES: u32 = 3;

/// A status read plus the one fact the wire shape cannot carry: whether the
/// repository holds unmerged records anywhere, which decides both
/// `committable` and which refusal a commit deserves.
pub(super) struct StatusCapture {
    pub status: SourceControlStatus,
    pub unmerged: bool,
}

/// Step 0 locates the root with one `rev-parse --show-prefix
/// --absolute-git-dir` (and, the first time this process has seen this git
/// dir, sweeps its stale discard quarantine directories), then the whole-
/// repository status is read together with the index identity it was taken
/// against, then Git decides `committable`, the index identity is computed
/// read-only, and the whole reviewed state is folded into one `snapshot_id`
/// against the git dir that same `rev-parse` resolved.
///
/// `mutationInProgress` is answered from the git dir that same `rev-parse`
/// returned, not from the configured root, so a root this process has never
/// mutated still sees the mutation a sibling root over the same worktree is
/// running; the resolution is handed to the registry so a later `acquire` on
/// this root skips its own probe.
pub(super) async fn capture_status(
    repo_root: &Path,
    cancel_token: Option<BundleCancelToken>,
    locks: &SourceControlLocks,
) -> Result<StatusCapture, AgentError> {
    let location = runner::capture_location(repo_root, cancel_token.clone()).await?;
    let prefix = location.prefix;
    if locks.mark_quarantine_swept(&location.git_dir) {
        sweep_stale_quarantine(&location.git_dir, locks).await;
    }
    let (output, index_tree_sha) =
        capture_stable_porcelain(repo_root, &prefix, cancel_token.clone()).await?;
    let index_ready = capture_index_ready(repo_root, cancel_token.clone()).await?;
    locks.remember_git_dir(repo_root, &location.git_dir);
    let mutation_in_progress = locks.is_busy_for_git_dir(&location.git_dir);
    let mut capture = project_status(
        &prefix,
        output,
        index_ready,
        index_tree_sha,
        mutation_in_progress,
    )?;
    capture.status.snapshot_id = capture_snapshot_id(&location.git_dir, &capture.status).await;
    Ok(StatusCapture {
        status: stamp_worktree_entries(repo_root, capture.status).await?,
        unmerged: capture.unmerged,
    })
}

/// Reads the whole-repository porcelain status together with the index
/// identity it was taken against: the identity is read before and after the
/// porcelain capture, and a mismatch means an external `git add` (or
/// equivalent) landed mid-read, so the read is retried. Still torn after the
/// bound: the porcelain lists stand (best effort, sorted and projected as
/// normal), but the identity is reported empty rather than pairing a stale
/// sha with fresher rows — a commit precondition bound to that value would
/// then authorize state nobody reviewed, and an empty identity refuses every
/// commit until a clean read succeeds.
async fn capture_stable_porcelain(
    repo_root: &Path,
    prefix: &[u8],
    cancel_token: Option<BundleCancelToken>,
) -> Result<(GitCommandOutput, String), AgentError> {
    resolve_stable_attempt(|attempt_number| {
        let cancel_token = cancel_token.clone();
        async move {
            let before = capture_index_tree_sha(repo_root, prefix, cancel_token.clone()).await?;
            let output = runner::run_read(repo_root, status_call(), cancel_token.clone()).await?;
            let after = capture_index_tree_sha(repo_root, prefix, cancel_token).await?;
            if before != after {
                log_torn_status(repo_root, attempt_number);
            }
            Ok(Attempt {
                identity_before: before,
                output,
                identity_after: after,
            })
        }
    })
    .await
}

/// One attempt at the identity/porcelain pair: the index identity read before
/// and after the porcelain capture, and the porcelain output itself.
struct Attempt {
    identity_before: String,
    output: GitCommandOutput,
    identity_after: String,
}

/// The retry policy alone, independent of how one attempt is read: up to
/// `TORN_STATUS_RETRIES` attempts, returning the first whose identity did not
/// move, or the last attempt's output paired with an empty identity when
/// every one moved. Factored out from `capture_stable_porcelain` so the tear
/// itself — hard to force from a real concurrent `git add` — can be exercised
/// directly with an injected sequence of attempts (see `tests` below).
async fn resolve_stable_attempt<F, Fut>(
    mut read_attempt: F,
) -> Result<(GitCommandOutput, String), AgentError>
where
    F: FnMut(u32) -> Fut,
    Fut: std::future::Future<Output = Result<Attempt, AgentError>>,
{
    // The first attempt is taken outside the loop so the torn result always has
    // an output to carry, without a `None` case no caller can reach and no
    // `expect` on a request path (ADR-008).
    let mut attempt = read_attempt(1).await?;
    for attempt_number in 2..=TORN_STATUS_RETRIES {
        if attempt.identity_before == attempt.identity_after {
            return Ok((attempt.output, attempt.identity_after));
        }
        attempt = read_attempt(attempt_number).await?;
    }
    if attempt.identity_before == attempt.identity_after {
        return Ok((attempt.output, attempt.identity_after));
    }
    Ok((attempt.output, String::new()))
}

fn status_call() -> GitCall {
    GitCall::new([
        "-c",
        "status.relativePaths=false",
        "status",
        "--porcelain=v2",
        "-z",
        "--branch",
        "--untracked-files=all",
        "--ignore-submodules=none",
    ])
    .stdout_limit(STATUS_LIMIT)
    .timeout(READ_TIMEOUT)
}

fn log_torn_status(repo_root: &Path, attempt: u32) {
    eprintln!(
        "{{\"level\":\"warn\",\"msg\":\"source control index identity changed while status was being read\",\"repoRoot\":{:?},\"attempt\":{attempt}}}",
        repo_root.display().to_string()
    );
}

/// Whether the index holds something a commit could record: it differs from
/// HEAD (the empty tree on an unborn branch), or a merge is being concluded,
/// which Git records even when the resolved tree equals HEAD. The projected
/// `index` list cannot tell either case from "nothing staged", so Git is asked
/// with two bounded reads; each accepts exit 1 as its "no" answer. Unresolved
/// conflicts are a separate dimension and are applied by the projector.
async fn capture_index_ready(
    repo_root: &Path,
    cancel_token: Option<BundleCancelToken>,
) -> Result<bool, AgentError> {
    let cached =
        GitCall::new(["diff", "--cached", "--quiet", "--no-ext-diff"]).accept_exit_codes(&[1]);
    let index = runner::run_read(repo_root, cached, cancel_token.clone()).await?;
    if index.exit_code == 1 {
        return Ok(true);
    }
    let probe = GitCall::new(["rev-parse", "-q", "--verify", "MERGE_HEAD"]).accept_exit_codes(&[1]);
    let merge_head = runner::run_read(repo_root, probe, cancel_token).await?;
    Ok(merge_head.exit_code == 0)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::{resolve_stable_attempt, Attempt, GitCommandOutput, TORN_STATUS_RETRIES};

    fn output(marker: &str) -> GitCommandOutput {
        GitCommandOutput {
            stdout: marker.as_bytes().to_vec(),
            stdout_truncated: false,
            stderr: Vec::new(),
            exit_code: 0,
        }
    }

    /// A stable pair on the very first attempt returns immediately and never
    /// asks for a second one.
    #[tokio::test]
    async fn a_stable_first_attempt_is_returned_without_a_retry() {
        let calls = AtomicU32::new(0);
        let (result, identity) = resolve_stable_attempt(|_| {
            calls.fetch_add(1, Ordering::SeqCst);
            async {
                Ok(Attempt {
                    identity_before: "same".to_string(),
                    output: output("first"),
                    identity_after: "same".to_string(),
                })
            }
        })
        .await
        .expect("resolves");
        assert_eq!(identity, "same");
        assert_eq!(result.stdout, b"first");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    /// Two torn attempts followed by a stable one stop retrying as soon as the
    /// identity holds, and report that third attempt's own output.
    #[tokio::test]
    async fn a_later_stable_attempt_is_returned_after_the_torn_ones() {
        let (result, identity) = resolve_stable_attempt(|attempt_number| async move {
            let torn = attempt_number < 3;
            Ok(Attempt {
                identity_before: "before".to_string(),
                output: output(&format!("attempt {attempt_number}")),
                identity_after: if torn { "after".to_string() } else { "before".to_string() },
            })
        })
        .await
        .expect("resolves");
        assert_eq!(identity, "before");
        assert_eq!(result.stdout, b"attempt 3");
    }

    /// Every attempt torn: the bound is respected (exactly
    /// `TORN_STATUS_RETRIES` reads), the last attempt's porcelain output is
    /// still returned best-effort, and the identity is empty rather than
    /// pairing a stale sha with it.
    #[tokio::test]
    async fn every_attempt_torn_reports_the_last_output_with_an_empty_identity() {
        let calls = AtomicU32::new(0);
        let (result, identity) = resolve_stable_attempt(|attempt_number| {
            calls.fetch_add(1, Ordering::SeqCst);
            async move {
                Ok(Attempt {
                    identity_before: "before".to_string(),
                    output: output(&format!("attempt {attempt_number}")),
                    identity_after: "after".to_string(),
                })
            }
        })
        .await
        .expect("resolves");
        assert_eq!(identity, "");
        assert_eq!(result.stdout, format!("attempt {TORN_STATUS_RETRIES}").as_bytes());
        assert_eq!(calls.load(Ordering::SeqCst), TORN_STATUS_RETRIES);
    }
}
