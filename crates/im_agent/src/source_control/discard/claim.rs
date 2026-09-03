// Path: crates/im_agent/src/source_control/discard/claim.rs
// Description: Per-target quarantine claim, verification, release, and rollback for discard

use std::path::{Path, PathBuf};

use im_bundle::fs_atomic::rename_no_replace;

use crate::error::{AgentError, MutationEffect};
use crate::protocol::SourceControlWorktreeStamp;

use super::quarantine::{claimed_file, hold_unrestored, mark_retained};
use crate::source_control::paths::ensure_within_root;
use crate::source_control::status::stamp::stamp_of;

/// One target moved into quarantine and verified to be exactly the file the
/// user reviewed. Consumed by `release` (the claim is superseded — its
/// content already restored from Git, or the removal itself) or `restore`
/// (put back exactly as claimed, because it turned out nothing should have
/// changed for this target after all).
pub(super) struct Claim {
    /// The operation directory holding these bytes, so the caller can record
    /// beside them what they were verified against.
    pub(super) root: PathBuf,
}

/// What claiming found: a file existed and matched, or there was nothing to
/// claim at all — the target's `expectedMissing` was verified true, or the
/// review made no assertion about this path's bytes and the path is indeed
/// absent (a rename's origin endpoint), which is only ever restored, never
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
///
/// The claim itself uses the plain rename: its destination is this
/// operation's own empty `claimed` slot, and requiring a no-replace rename
/// here would refuse every discard on a filesystem that has none.
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
        return Ok(Claim {
            root: quarantine_root.to_path_buf(),
        });
    }
    Err(roll_back(&quarantined, &source, path))
}

/// The claim's own rename failed, so nothing moved. A worktree on a different
/// volume from its own repository can never be claimed at all: no rename will
/// ever move a file between them, and that is a layout the user has to change,
/// not a state that will settle on its own. A target that is simply gone by
/// now is the ordinary "changed since it was reviewed" refusal — the file the
/// user reviewed was deleted between the review and this click, and reporting
/// that as an internal failure tells them nothing they can act on. Any other
/// reason is still ours to report as internal.
fn unclaimable(path: &str, source: &Path, error: &std::io::Error) -> ClaimFailure {
    if error.kind() == std::io::ErrorKind::CrossesDevices {
        return ClaimFailure::Refused(AgentError::new(
            "SOURCE_CONTROL_UNSUPPORTED_LAYOUT",
            format!(
                "Discard cannot claim {path}: the worktree and its repository live on different volumes"
            ),
        ));
    }
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
/// effect is unknown and the message names where the bytes are held.
fn roll_back(quarantined: &Path, source: &Path, path: &str) -> ClaimFailure {
    match put_back_bytes(quarantined, source, path, "after a discard verification mismatch") {
        Ok(()) => ClaimFailure::Refused(state_changed(path)),
        Err(error) => ClaimFailure::EffectUnknown(error),
    }
}

/// Returns claimed bytes to the worktree without ever replacing what is there
/// now. A discard is allowed to destroy exactly the file the user confirmed;
/// putting bytes back is not allowed to destroy anything at all, so a
/// destination that has been written since the claim, or a filesystem that
/// cannot move a file without replacing the destination, stops the put-back
/// rather than overwriting a stranger's work.
fn put_back_bytes(
    quarantined: &Path,
    source: &Path,
    path: &str,
    when: &str,
) -> Result<(), AgentError> {
    let Err(error) = rename_no_replace(quarantined, source) else {
        return Ok(());
    };
    match error.kind() {
        std::io::ErrorKind::AlreadyExists => {
            Err(held(quarantined, format!("a newer file appeared at {path}")))
        }
        std::io::ErrorKind::Unsupported => Err(held(
            quarantined,
            "the filesystem cannot rename without replacing".to_string(),
        )),
        _ => Err(held(
            quarantined,
            format!("Failed to restore {path} {when}: {error}"),
        )),
    }
}

/// A claimed file could not be put back where it came from, whichever step
/// wanted it back: the bytes the user reviewed are the only copy left and they
/// are sitting in quarantine. They are renamed out of the sweep's reach, and
/// the failure names the reason and where they are held so the user can go and
/// get them. The effect is unknown by construction — this target's file is not
/// where either outcome says it should be.
fn held(quarantined: &Path, reason: String) -> AgentError {
    let held = hold_unrestored(quarantined);
    AgentError::internal(format!(
        "{reason}; the reviewed bytes are held at {}",
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

impl Claim {
    /// The Git restore or removal landed: the claimed copy is superseded and
    /// becomes this operation's retained bytes. A retention that cannot rename
    /// would leave those bytes under the one name the sweep finishes — beside
    /// the marker that authorizes it — so they are held instead.
    pub(super) fn release(self) -> Result<(), AgentError> {
        let Err(error) = mark_retained(&self.root) else {
            return Ok(());
        };
        Err(self.hold(format!("Could not retain the discarded bytes: {error}")))
    }

    /// Something after the `verified` marker failed. The worktree path is
    /// already empty, the claimed bytes are the only copy of what the user
    /// reviewed, and the marker beside them tells the next start's sweep it
    /// may destroy them — which is exactly what nobody authorized once the
    /// step that marker announced did not land. They are renamed out of that
    /// sweep's reach and the failure names where they are.
    pub(super) fn hold(self, reason: String) -> AgentError {
        held(&claimed_file(&self.root), reason)
    }

    /// Nothing should have changed for this target after all (its status
    /// classification came back unlisted): put it back exactly as claimed.
    /// A rename that fails here strands the reviewed bytes exactly as a failed
    /// rollback does, and is reported the same way.
    pub(super) fn restore(self, repo_root: &Path, path: &str) -> Result<(), AgentError> {
        put_back_bytes(
            &claimed_file(&self.root),
            &repo_root.join(path),
            path,
            "after its discard turned out to need no change",
        )
    }
}

#[cfg(test)]
mod tests {
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

    /// Someone wrote the original path while the claim was out of the
    /// worktree. The rollback must not put the reviewed bytes back over that
    /// newer file: it holds them instead, names the reason, and reports an
    /// unknown effect rather than the clean refusal a rollback usually earns.
    #[test]
    fn a_rollback_onto_a_newer_file_holds_the_reviewed_bytes_instead_of_replacing_it() {
        let temp = tempfile::tempdir().expect("tempdir");
        let quarantine_root = temp.path().join("op");
        std::fs::create_dir_all(&quarantine_root).expect("op dir");
        let quarantined = claimed_file(&quarantine_root);
        std::fs::write(&quarantined, b"reviewed\n").expect("claimed file");
        let source = temp.path().join("victim.txt");
        std::fs::write(&source, b"a coding agent wrote this\n").expect("newer file");

        let failure = roll_back(&quarantined, &source, "victim.txt");

        let ClaimFailure::EffectUnknown(error) = failure else {
            panic!("a rollback blocked by a newer file is not a proven-safe refusal");
        };
        assert_eq!(error.effect(), Some("unknown"));
        assert!(
            error.message().starts_with("a newer file appeared at victim.txt"),
            "the message must name the reason: {}",
            error.message()
        );
        assert_eq!(
            std::fs::read(&source).expect("newer file"),
            b"a coding agent wrote this\n",
            "the newer file is never replaced by the bytes being put back"
        );
        assert_eq!(
            std::fs::read(quarantine_root.join("unrestored")).expect("held file"),
            b"reviewed\n"
        );
    }
}
