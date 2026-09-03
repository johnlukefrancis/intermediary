// Path: crates/im_agent/src/source_control/actions_discard_claim.rs
// Description: Atomic per-target quarantine claim, verification, release, and rollback for discard

use std::path::{Path, PathBuf};

use crate::error::{AgentError, MutationEffect};
use crate::protocol::SourceControlWorktreeStamp;

use super::discard_quarantine::{claimed_file, hold_unrestored};
use super::paths::ensure_within_root;
use super::status_stamp::stamp_of;

/// One target claimed into quarantine and verified to be exactly the file the
/// user reviewed. Consumed by `release` (the claim is superseded — its
/// content already restored from Git, or the removal itself) or `restore`
/// (put back exactly as claimed, because it turned out nothing should have
/// changed for this target after all).
pub(super) struct Claim {
    quarantined: PathBuf,
}

/// What claiming found: a file existed and matched, or there was nothing to
/// claim at all — the target's `expectedMissing` was verified true, or the
/// review made no assertion about this path's bytes (a rename's second
/// endpoint, or a non-regular file), which is only ever restored, never
/// removed.
pub(super) enum ClaimOutcome {
    Nothing,
    Claimed(Claim),
}

/// How a claim attempt failed. `Refused` is proven safe: the file, if it
/// moved at all, is already back where it started. `EffectUnknown` is not —
/// the file was claimed and could not be put back, so the reviewed bytes are
/// held in quarantine and this target's state is no longer provably
/// untouched.
pub(super) enum ClaimFailure {
    Refused(AgentError),
    EffectUnknown(AgentError),
}

/// Renames the target into `quarantine_root` (creating it if needed) and
/// verifies the quarantined copy's stamp against `expected`; a mismatch is
/// rolled back (renamed back) before the refusal is returned, so the
/// worktree is untouched whenever this returns `Refused`. Only a rollback
/// that itself fails leaves anything moved, and that is reported as its own
/// failure rather than as a refusal.
pub(super) fn claim_existing(
    repo_root: &Path,
    quarantine_root: &Path,
    path: &str,
    expected: SourceControlWorktreeStamp,
) -> Result<Claim, ClaimFailure> {
    ensure_within_root(repo_root, path).map_err(ClaimFailure::Refused)?;
    let source = repo_root.join(path);
    let quarantined = claimed_file(quarantine_root);
    std::fs::create_dir_all(quarantine_root).map_err(|error| {
        ClaimFailure::Refused(AgentError::internal(format!(
            "Could not prepare a discard quarantine directory: {error}"
        )))
    })?;
    if let Err(error) = std::fs::rename(&source, &quarantined) {
        return Err(unclaimable(path, &source, &error));
    }
    if stamp_of(&quarantined).stamp == Some(expected) {
        return Ok(Claim { quarantined });
    }
    Err(roll_back(&quarantined, &source, path))
}

/// The claim's own rename failed, so nothing moved. A target that is simply
/// gone by now is the ordinary "changed since it was reviewed" refusal — the
/// file the user reviewed was deleted between the review and this click, and
/// reporting that as an internal failure tells them nothing they can act on.
/// Any other reason is still ours to report as internal.
fn unclaimable(path: &str, source: &Path, error: &std::io::Error) -> ClaimFailure {
    if stamp_of(source).missing {
        return ClaimFailure::Refused(state_changed(path));
    }
    ClaimFailure::Refused(AgentError::internal(format!(
        "Could not claim {path} for discard: {error}"
    )))
}

/// Verification failed, so this target must end exactly where it started.
/// When the file cannot be put back, the bytes the user reviewed are stranded
/// in quarantine: that is no longer a refusal anyone can call safe, so the
/// effect is unknown, the message names where the bytes are held, and the
/// claim is renamed out of the sweep's reach rather than left for a later
/// process to finish destroying.
fn roll_back(quarantined: &Path, source: &Path, path: &str) -> ClaimFailure {
    let Err(error) = std::fs::rename(quarantined, source) else {
        return ClaimFailure::Refused(state_changed(path));
    };
    ClaimFailure::EffectUnknown(stranded(
        quarantined,
        path,
        "after a discard verification mismatch",
        &error,
    ))
}

/// A claimed file could not be put back where it came from, whichever step
/// wanted it back: the bytes the user reviewed are the only copy left and they
/// are sitting in quarantine. They are renamed out of the sweep's reach, and
/// the failure names where they are held so the user can go and get them. The
/// effect is unknown by construction — this target's file is not where either
/// outcome says it should be.
fn stranded(quarantined: &Path, path: &str, when: &str, error: &std::io::Error) -> AgentError {
    let held = hold_unrestored(quarantined);
    AgentError::internal(format!(
        "Failed to restore {path} {when}: {error}; \
         the reviewed content of {path} is held at {}",
        held.display()
    ))
    .with_effect(MutationEffect::Unknown)
}

fn state_changed(path: &str) -> AgentError {
    AgentError::new(
        "SOURCE_CONTROL_STATE_CHANGED",
        format!("{path} changed since it was reviewed"),
    )
}

/// The file must still be absent from `path`: reviewed as `worktreeMissing`,
/// it is refused instead of restored-over if a newer file has since appeared.
pub(super) fn verify_still_missing(repo_root: &Path, path: &str) -> Result<(), ClaimFailure> {
    if stamp_of(&repo_root.join(path)).missing {
        return Ok(());
    }
    Err(ClaimFailure::Refused(AgentError::new(
        "SOURCE_CONTROL_STATE_CHANGED",
        format!("{path} was created after it was reviewed"),
    )))
}

impl Claim {
    /// The claimed copy is superseded and can be discarded for good.
    pub(super) fn release(self) -> std::io::Result<()> {
        match std::fs::remove_file(&self.quarantined) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }

    /// Nothing should have changed for this target after all (its status
    /// classification came back unlisted): put it back exactly as claimed.
    /// A rename that fails here strands the reviewed bytes exactly as a failed
    /// rollback does, and is reported the same way.
    pub(super) fn restore(self, repo_root: &Path, path: &str) -> Result<(), AgentError> {
        let Err(error) = std::fs::rename(&self.quarantined, repo_root.join(path)) else {
            return Ok(());
        };
        Err(stranded(
            &self.quarantined,
            path,
            "after its discard turned out to need no change",
            &error,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::discard_quarantine::{quarantine_root, sweep_stale_quarantine};
    use super::*;

    /// The rollback cannot put the file back (the directory it came from is
    /// gone by now): the claim is no longer a refusal anyone can call safe.
    /// The effect is unknown, the message names where the reviewed bytes are
    /// held, and the bytes are renamed out of the sweep's reach.
    #[test]
    fn a_rollback_that_cannot_put_the_file_back_holds_the_reviewed_bytes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let quarantine_root = temp.path().join("op");
        std::fs::create_dir_all(&quarantine_root).expect("op dir");
        let quarantined = claimed_file(&quarantine_root);
        std::fs::write(&quarantined, b"reviewed\n").expect("claimed file");
        let source = temp.path().join("gone").join("victim.txt");

        let failure = roll_back(&quarantined, &source, "gone/victim.txt");

        let ClaimFailure::EffectUnknown(error) = failure else {
            panic!("a rollback that failed is never a proven-safe refusal");
        };
        assert_eq!(error.effect(), Some("unknown"));
        let held = quarantine_root.join("unrestored");
        assert!(
            error.message().contains(&held.display().to_string()),
            "the message must name where the bytes are: {}",
            error.message()
        );
        assert_eq!(std::fs::read(&held).expect("held file"), b"reviewed\n");
        assert!(
            !quarantined.exists(),
            "the claim must stop looking authorized for destruction"
        );
        assert!(!source.exists(), "the worktree path stays as the failure left it");
    }

    /// The put-back has the same stake as the rollback: the claimed file is
    /// the only copy of what the user reviewed, and this target was never
    /// authorized for destruction at all. A rename that cannot land must
    /// leave those bytes standing — including past the sweep that finishes
    /// authorized discards.
    #[tokio::test]
    async fn a_put_back_that_cannot_land_holds_the_reviewed_bytes_past_the_sweep() {
        let temp = tempfile::tempdir().expect("tempdir");
        let git_dir = temp.path().join("git");
        let quarantine = quarantine_root(&git_dir, "op-id");
        std::fs::create_dir_all(&quarantine).expect("op dir");
        let quarantined = claimed_file(&quarantine);
        std::fs::write(&quarantined, b"reviewed\n").expect("claimed file");
        // The worktree the file came from is gone, so the rename cannot land.
        let repo_root = temp.path().join("repo");

        let error = Claim {
            quarantined: quarantined.clone(),
        }
        .restore(&repo_root, "victim.txt")
        .expect_err("the put-back cannot land");

        assert_eq!(error.effect(), Some("unknown"));
        let held = quarantine.join("unrestored");
        assert!(
            error.message().contains(&held.display().to_string()),
            "the message must name where the bytes are: {}",
            error.message()
        );
        assert!(!quarantined.exists());

        sweep_stale_quarantine(&git_dir).await;
        assert_eq!(
            std::fs::read(&held).expect("held file"),
            b"reviewed\n",
            "the sweep spares content no rollback or put-back could restore"
        );
    }
}
