// Path: crates/im_bundle/src/git_capture/finalize.rs
// Description: Git artifact finalization and working-tree coherence verdicts

use std::collections::HashSet;

use crate::cancel::BundleCancelToken;
use crate::error::Result;

use super::diff::{capture_diff_artifacts, recapture_patch};
use super::index::capture_index_tree_sha;
use super::render::{handoff_note, render_status};
use super::render_omitted::render_omitted_paths;
use super::status::{ParsedStatus, SelectedStatusRecord};
use super::verification::{verify_written_state, WrittenEntryDigests};
use super::{
    CapturedGitEvidence, GitCaptureIssue, GitCaptureSession, GitCaptureState, GIT_DIFF_NAME,
    GIT_STATUS_NAME,
};

impl GitCaptureSession {
    pub(crate) fn finish(
        mut self,
        written_digests: &WrittenEntryDigests,
        cancel_token: Option<&BundleCancelToken>,
    ) -> Result<CapturedGitEvidence> {
        let mut patch = Vec::new();
        let mut index_patch = Vec::new();
        let mut worktree_patch = Vec::new();
        let mut stat = Vec::new();
        let mut name_status = Vec::new();
        if let (Some(status), Some(head_sha)) =
            (self.parsed_status.clone(), self.manifest.head_sha.clone())
        {
            if self.manifest.status != GitCaptureState::Unavailable {
                let artifacts = capture_diff_artifacts(
                    &self.config,
                    &status,
                    &head_sha,
                    self.manifest.patch_deletions,
                    cancel_token,
                )?;
                patch = artifacts.patch;
                index_patch = artifacts.index_patch;
                worktree_patch = artifacts.worktree_patch;
                stat = artifacts.stat;
                name_status = artifacts.name_status;
                let patch_complete = !artifacts
                    .issues
                    .iter()
                    .any(|issue| issue.artifact.as_deref() == Some(GIT_DIFF_NAME));
                if !artifacts.issues.is_empty() {
                    self.mark_partial();
                    self.manifest.issues.extend(artifacts.issues);
                }
                self.verify_capture(
                    &status,
                    &head_sha,
                    &patch,
                    patch_complete,
                    written_digests,
                    cancel_token,
                )?;
            }
        }
        self.finalize_incomplete_artifacts();
        let status = render_status(&self.manifest, self.records(), &stat, &name_status);
        let omitted_paths = render_omitted_paths(
            self.parsed_status
                .as_ref()
                .map(|status| status.omitted.as_slice()),
        );
        Ok(CapturedGitEvidence {
            manifest: self.manifest,
            status,
            diff: patch,
            index_diff: index_patch,
            worktree_diff: worktree_patch,
            omitted_paths,
            handoff: handoff_note().to_vec(),
        })
    }

    fn verify_capture(
        &mut self,
        status: &ParsedStatus,
        head_sha: &str,
        patch: &[u8],
        patch_complete: bool,
        written_digests: &WrittenEntryDigests,
        cancel_token: Option<&BundleCancelToken>,
    ) -> Result<()> {
        if self
            .initial_patch
            .as_ref()
            .is_some_and(|initial_patch| initial_patch != patch)
        {
            self.mark_unstable(
                "captureDrift",
                Some(GIT_DIFF_NAME),
                "The selected HEAD delta moved after initial capture and before archive finalization.",
            );
        }
        if self.initial_digests_complete
            && status
                .watched_regular_paths
                .iter()
                .any(|path| self.initial_digests.get(path) != written_digests.get(path))
        {
            self.mark_unstable(
                "captureDrift",
                Some(GIT_DIFF_NAME),
                "Selected file bytes moved between initial capture and archive writing.",
            );
        }
        if patch_complete {
            let second_patch = recapture_patch(
                &self.config,
                status,
                head_sha,
                self.manifest.patch_deletions,
                cancel_token,
            )?;
            match second_patch {
                Ok(second_patch) if second_patch != patch => self.mark_unstable(
                    "captureDrift",
                    Some(GIT_DIFF_NAME),
                    "The selected HEAD delta changed while Git evidence was captured.",
                ),
                Ok(_) => {}
                Err(issue) => {
                    self.mark_partial();
                    self.manifest.issues.push(issue);
                }
            }
        }
        let verification = verify_written_state(
            &self.config.repo_root,
            &status.watched_regular_paths,
            written_digests,
            cancel_token,
        )?;
        if verification.drifted {
            self.manifest.status = GitCaptureState::Unstable;
        }
        if verification
            .issues
            .iter()
            .any(|issue| issue.kind == "verificationTimeout")
        {
            self.mark_partial();
        }
        self.manifest.issues.extend(verification.issues);
        self.verify_index_tree(cancel_token)?;
        self.verify_selected_ignored(cancel_token)?;
        self.verify_status_digest(cancel_token)
    }

    fn verify_index_tree(&mut self, cancel_token: Option<&BundleCancelToken>) -> Result<()> {
        let Some(initial) = self.initial_index_tree_sha.clone() else {
            return Ok(());
        };
        match capture_index_tree_sha(&self.config, cancel_token)? {
            Ok(current) if current == initial => {}
            Ok(_) => self.mark_unstable(
                "captureDrift",
                None,
                "The index changed while bundle evidence was captured; candidateIndexTreeSha is the initial value.",
            ),
            Err(issue) => {
                self.mark_partial();
                self.manifest.issues.push(issue);
            }
        }
        Ok(())
    }

    fn verify_status_digest(&mut self, cancel_token: Option<&BundleCancelToken>) -> Result<()> {
        let expected = self.pre_status_digest;
        match self.capture_status(cancel_token)? {
            Ok((_status, digest)) if Some(digest) == expected => {}
            Ok(_) => self.mark_unstable(
                "captureDrift",
                Some(GIT_STATUS_NAME),
                "HEAD or repository status moved while bundle evidence was captured.",
            ),
            Err(issue) => {
                self.mark_partial();
                self.manifest.issues.push(issue);
            }
        }
        Ok(())
    }

    fn records(&self) -> &[SelectedStatusRecord] {
        self.parsed_status
            .as_ref()
            .map(|status| status.selected_records.as_slice())
            .unwrap_or(&[])
    }

    pub(super) fn mark_partial(&mut self) {
        if self.manifest.status == GitCaptureState::Complete {
            self.manifest.status = GitCaptureState::Partial;
        }
    }

    pub(super) fn mark_unstable(&mut self, kind: &str, artifact: Option<&str>, detail: &str) {
        self.manifest.status = GitCaptureState::Unstable;
        self.manifest
            .issues
            .push(GitCaptureIssue::new(kind, artifact, detail));
    }

    fn finalize_incomplete_artifacts(&mut self) {
        let mut names: HashSet<String> = self
            .manifest
            .issues
            .iter()
            .filter_map(|issue| issue.artifact.clone())
            .collect();
        if self.manifest.status == GitCaptureState::Unavailable
            || self.manifest.status == GitCaptureState::Unstable
        {
            names.insert(GIT_STATUS_NAME.to_string());
            names.insert(GIT_DIFF_NAME.to_string());
        }
        if self.manifest.status == GitCaptureState::Partial && self.parsed_status.is_none() {
            names.insert(GIT_DIFF_NAME.to_string());
        }
        let mut names: Vec<_> = names.into_iter().collect();
        names.sort();
        self.manifest.incomplete_artifacts = names;
    }
}
