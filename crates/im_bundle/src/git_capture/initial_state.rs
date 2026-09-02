// Path: crates/im_bundle/src/git_capture/initial_state.rs
// Description: Initial selected-delta, index-tree, and file fingerprint capture for a Git session

use crate::cancel::BundleCancelToken;
use crate::error::Result;

use super::diff::recapture_patch;
use super::index::capture_index_tree_sha;
use super::verification::capture_current_digests;
use super::{GitCaptureIssue, GitCaptureSession, GitCaptureState, PatchDeletions, GIT_DIFF_NAME};

impl GitCaptureSession {
    pub(super) fn capture_initial_state(
        &mut self,
        cancel_token: Option<&BundleCancelToken>,
    ) -> Result<()> {
        let (Some(status), Some(head_sha)) =
            (self.parsed_status.clone(), self.manifest.head_sha.clone())
        else {
            return Ok(());
        };
        if self.manifest.status == GitCaptureState::Unavailable {
            return Ok(());
        }
        // Full deletion bodies are the ordinary review evidence. Only when a
        // patch carrying them overruns the reviewable budget (or the hard
        // bound) does capture retry header-only, keeping the delta complete
        // and readable instead of truncated or dominated by removed content.
        let mut captured = recapture_patch(
            &self.config,
            &status,
            &head_sha,
            PatchDeletions::Full,
            cancel_token,
        )?;
        let oversized = match &captured {
            Ok(patch) => patch.len() > self.config.full_deletions_budget,
            Err(issue) => issue.kind == "outputTruncated",
        };
        if oversized && status.counts.selected_deleted > 0 {
            let header_only = recapture_patch(
                &self.config,
                &status,
                &head_sha,
                PatchDeletions::HeaderOnly,
                cancel_token,
            )?;
            if header_only.is_ok() {
                self.manifest.patch_deletions = PatchDeletions::HeaderOnly;
                captured = header_only;
            }
        }
        match captured {
            Ok(patch) => self.initial_patch = Some(patch),
            Err(issue) => {
                self.mark_initial_partial();
                self.manifest.issues.push(issue);
            }
        }
        match capture_index_tree_sha(&self.config, cancel_token)? {
            Ok(sha) => {
                self.initial_index_tree_sha = Some(sha.clone());
                self.manifest.candidate_index_tree_sha = Some(sha);
            }
            Err(issue) => {
                self.mark_initial_partial();
                self.manifest.issues.push(issue);
            }
        }
        let captured = capture_current_digests(
            &self.config.repo_root,
            &status.watched_regular_paths,
            cancel_token,
        )?;
        self.initial_digests = captured.digests;
        self.initial_digests_complete = captured.complete;
        if !captured.complete {
            self.mark_initial_partial();
            self.manifest.issues.push(GitCaptureIssue::new(
                if captured.timed_out {
                    "verificationTimeout"
                } else {
                    "stateReadFailure"
                },
                Some(GIT_DIFF_NAME),
                "The initial selected-file state could not be fully fingerprinted within capture bounds.",
            ));
        }
        Ok(())
    }
}
