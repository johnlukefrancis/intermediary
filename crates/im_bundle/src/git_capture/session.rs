// Path: crates/im_bundle/src/git_capture/session.rs
// Description: Git capture discovery, initial status, and safety-bound setup

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::cancel::BundleCancelToken;
use crate::error::Result;
use crate::plan::BundlePlan;
use crate::selection::BundleSelector;

use super::command::run_git;
use super::diff::{common_git_args, FULL_DELETIONS_BUDGET, PATCH_LIMIT};
use super::discovery::{initial_issue, trim_line_ending};
use super::status::{parse_status, ParsedStatus};
use super::{
    empty_manifest, GitCaptureConfig, GitCaptureIssue, GitCaptureSession, GitCaptureState,
    GIT_DIFF_NAME, GIT_STATUS_NAME,
};

const STATUS_LIMIT: usize = 8 * 1024 * 1024;
const PREFIX_LIMIT: usize = 1024 * 1024;
const PATH_COUNT_LIMIT: usize = 16384;
const PATH_BYTES_LIMIT: usize = 1024 * 1024;

impl GitCaptureSession {
    pub(crate) fn begin(
        plan: &BundlePlan,
        cancel_token: Option<&BundleCancelToken>,
    ) -> Result<Self> {
        let config = GitCaptureConfig {
            executable: PathBuf::from("git"),
            repo_root: plan.repo_root.clone(),
            command_timeout: Duration::from_secs(5),
            patch_limit: PATCH_LIMIT,
            full_deletions_budget: FULL_DELETIONS_BUDGET,
        };
        Self::begin_with_config(plan, config, cancel_token)
    }

    pub(super) fn begin_with_config(
        plan: &BundlePlan,
        config: GitCaptureConfig,
        cancel_token: Option<&BundleCancelToken>,
    ) -> Result<Self> {
        let selector = BundleSelector::new(&plan.selection, &plan.global_excludes)?;
        let mut session = Self {
            config,
            selector,
            repo_prefix: Vec::new(),
            pre_status_digest: None,
            initial_patch: None,
            initial_index_tree_sha: None,
            initial_digests: Default::default(),
            initial_digests_complete: false,
            selected_file_input: None,
            selected_file_paths: Default::default(),
            initial_ignored_paths: None,
            parsed_status: None,
            manifest: empty_manifest(),
        };

        let prefix = match session.capture_prefix(cancel_token)? {
            Ok(prefix) => prefix,
            Err(issue) => {
                session.manifest.issues.push(issue);
                return Ok(session);
            }
        };
        session.repo_prefix = prefix;
        match session.capture_status(cancel_token)? {
            Ok((parsed, digest)) => {
                session.apply_status_facts(&parsed);
                session.pre_status_digest = Some(digest);
                session.parsed_status = Some(parsed);
            }
            Err(issue) => {
                session.manifest.status = if issue.kind == "outputTruncated" {
                    GitCaptureState::Partial
                } else {
                    GitCaptureState::Unavailable
                };
                session.manifest.issues.push(issue);
                return Ok(session);
            }
        }

        if session.manifest.head_sha.is_none() {
            session.manifest.status = GitCaptureState::Unavailable;
            session.manifest.issues.push(GitCaptureIssue::new(
                "headUnavailable",
                Some(GIT_DIFF_NAME),
                "The repository has no captured HEAD commit to compare against.",
            ));
        } else if session.pathspec_limit_reached() {
            session.manifest.status = GitCaptureState::Partial;
            session.manifest.issues.push(GitCaptureIssue::new(
                "pathLimit",
                Some(GIT_DIFF_NAME),
                "Selected changed paths exceeded the bounded Git pathspec budget; the patch is incomplete.",
            ));
            if let Some(status) = &mut session.parsed_status {
                status.general_pathspecs.clear();
                status.rename_pathspecs.clear();
                status.watched_regular_paths.clear();
            }
        }
        session.capture_initial_state(cancel_token)?;
        Ok(session)
    }

    pub(crate) fn watched_paths(&self) -> HashSet<PathBuf> {
        self.parsed_status
            .as_ref()
            .map(|status| status.watched_regular_paths.clone())
            .unwrap_or_default()
    }

    pub(super) fn capture_status(
        &self,
        cancel_token: Option<&BundleCancelToken>,
    ) -> Result<std::result::Result<(ParsedStatus, [u8; 32]), GitCaptureIssue>> {
        let mut args = common_git_args();
        args.extend([
            "-c".into(),
            "status.relativePaths=false".into(),
            "status".into(),
            "--porcelain=v2".into(),
            "-z".into(),
            "--branch".into(),
            "--untracked-files=all".into(),
            "--ignore-submodules=none".into(),
        ]);
        let output = run_git(
            &self.config.executable,
            &self.config.repo_root,
            &args,
            STATUS_LIMIT,
            self.config.command_timeout,
            cancel_token,
        )?;
        let output = match output {
            Ok(output) => output,
            Err(failure) => return Ok(Err(initial_issue(failure))),
        };
        if output.stdout_truncated {
            return Ok(Err(GitCaptureIssue::new(
                "outputTruncated",
                Some(GIT_STATUS_NAME),
                "Git status exceeded its bounded output budget; selected counts and paths are incomplete.",
            )));
        }
        let digest: [u8; 32] = Sha256::digest(&output.stdout).into();
        let parsed = parse_status(
            &output.stdout,
            &self.repo_prefix,
            &self.config.repo_root,
            &self.selector,
        )
        .map_err(|_| {
            GitCaptureIssue::new(
                "statusParseFailure",
                Some(GIT_STATUS_NAME),
                "Git returned porcelain status data that could not be parsed safely.",
            )
        });
        Ok(parsed.map(|status| (status, digest)))
    }

    fn capture_prefix(
        &self,
        cancel_token: Option<&BundleCancelToken>,
    ) -> Result<std::result::Result<Vec<u8>, GitCaptureIssue>> {
        let mut args = common_git_args();
        args.extend([OsString::from("rev-parse"), OsString::from("--show-prefix")]);
        let output = run_git(
            &self.config.executable,
            &self.config.repo_root,
            &args,
            PREFIX_LIMIT,
            self.config.command_timeout,
            cancel_token,
        )?;
        let output = match output {
            Ok(output) => output,
            Err(failure) => return Ok(Err(initial_issue(failure))),
        };
        if output.stdout_truncated {
            return Ok(Err(GitCaptureIssue::new(
                "outputTruncated",
                Some(GIT_STATUS_NAME),
                "The Git repository-prefix result exceeded its safety bound.",
            )));
        }
        Ok(Ok(trim_line_ending(output.stdout)))
    }

    fn apply_status_facts(&mut self, status: &ParsedStatus) {
        self.manifest.status = GitCaptureState::Complete;
        self.manifest.head_sha = status.head_sha.clone();
        self.manifest.short_sha = status
            .head_sha
            .as_ref()
            .map(|sha| sha.chars().take(7).collect());
        self.manifest.branch = status.branch.clone();
        self.manifest.repo_dirty = Some(status.repo_dirty);
        self.manifest.selection_dirty = Some(status.selection_dirty);
        self.manifest.counts = status.counts.clone();
    }

    fn pathspec_limit_reached(&self) -> bool {
        let Some(status) = &self.parsed_status else {
            return false;
        };
        let paths = status
            .general_pathspecs
            .iter()
            .chain(status.rename_pathspecs.iter().flatten());
        let (count, bytes) = paths.fold((0usize, 0usize), |(count, bytes), path| {
            (count + 1, bytes.saturating_add(path.as_bytes().len()))
        });
        count > PATH_COUNT_LIMIT || bytes > PATH_BYTES_LIMIT
    }

    pub(super) fn mark_initial_partial(&mut self) {
        if self.manifest.status == GitCaptureState::Complete {
            self.manifest.status = GitCaptureState::Partial;
        }
    }
}
