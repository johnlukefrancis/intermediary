// Path: crates/im_bundle/src/git_capture/ignored.rs
// Description: Reconcile selected archived files that Git status hides behind ignore rules

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::PathBuf;

use crate::cancel::BundleCancelToken;
use crate::error::Result;
use crate::scanner::ScanEntry;

use super::command::{run_git_with_input, GitCommandFailure};
use super::diff::common_git_config_args;
use super::path::{path_to_bytes, GitPath};
use super::verification::capture_current_digests;
use super::{GitCaptureIssue, GitCaptureSession, GitCaptureState, GIT_STATUS_NAME};

const SELECTED_INPUT_LIMIT: usize = 8 * 1024 * 1024;
const SELECTED_FILE_COUNT_LIMIT: usize = 65_536;
const IGNORED_OUTPUT_LIMIT: usize = 8 * 1024 * 1024;

impl GitCaptureSession {
    pub(crate) fn reconcile_selected_files(
        &mut self,
        entries: &[ScanEntry],
        cancel_token: Option<&BundleCancelToken>,
    ) -> Result<()> {
        if self.parsed_status.is_none()
            || self.manifest.head_sha.is_none()
            || self.manifest.status == GitCaptureState::Unavailable
        {
            return Ok(());
        }

        let (input, selected_paths) = match encode_selected_files(entries) {
            Ok(encoded) => encoded,
            Err(issue) => {
                self.mark_initial_partial();
                self.manifest.issues.push(issue);
                return Ok(());
            }
        };
        let ignored_paths =
            match capture_selected_ignored(&self.config, &input, &selected_paths, cancel_token)? {
                Ok(paths) => paths,
                Err(issue) => {
                    self.mark_initial_partial();
                    self.manifest.issues.push(issue);
                    return Ok(());
                }
            };

        if let Some(status) = &mut self.parsed_status {
            status.add_ignored_untracked(&ignored_paths);
            self.manifest.selection_dirty = Some(status.selection_dirty);
            self.manifest.counts = status.counts.clone();
        }

        let watched_paths: HashSet<PathBuf> = ignored_paths
            .iter()
            .filter_map(GitPath::to_path_buf)
            .collect();
        let captured =
            capture_current_digests(&self.config.repo_root, &watched_paths, cancel_token)?;
        self.initial_digests.extend(captured.digests);
        self.initial_digests_complete &= captured.complete;
        if !captured.complete {
            self.mark_initial_partial();
            self.manifest.issues.push(GitCaptureIssue::new(
                if captured.timed_out {
                    "verificationTimeout"
                } else {
                    "stateReadFailure"
                },
                Some(GIT_STATUS_NAME),
                "Selected ignored-file bytes could not be fully fingerprinted within capture bounds.",
            ));
        }

        self.selected_file_input = Some(input);
        self.selected_file_paths = selected_paths;
        self.initial_ignored_paths = Some(ignored_paths);
        Ok(())
    }

    pub(super) fn verify_selected_ignored(
        &mut self,
        cancel_token: Option<&BundleCancelToken>,
    ) -> Result<()> {
        let (Some(input), Some(initial_paths)) = (
            self.selected_file_input.as_ref(),
            self.initial_ignored_paths.as_ref(),
        ) else {
            return Ok(());
        };
        match capture_selected_ignored(
            &self.config,
            input,
            &self.selected_file_paths,
            cancel_token,
        )? {
            Ok(paths) if &paths == initial_paths => {}
            Ok(_) => self.mark_unstable(
                "captureDrift",
                Some(GIT_STATUS_NAME),
                "Git ignore classification for selected archived files moved during capture.",
            ),
            Err(issue) => {
                self.mark_partial();
                self.manifest.issues.push(issue);
            }
        }
        Ok(())
    }
}

fn encode_selected_files(
    entries: &[ScanEntry],
) -> std::result::Result<(Vec<u8>, HashSet<GitPath>), GitCaptureIssue> {
    let mut input = Vec::new();
    let mut selected_paths = HashSet::new();
    for entry in entries {
        if selected_paths.len() >= SELECTED_FILE_COUNT_LIMIT {
            return Err(path_limit_issue());
        }
        let Some(path) = path_to_bytes(&entry.repo_relative_path) else {
            return Err(GitCaptureIssue::new(
                "unsupportedPathEncoding",
                Some(GIT_STATUS_NAME),
                "At least one selected archive path could not be passed losslessly to Git on this host.",
            ));
        };
        let required = path.len().saturating_add(1);
        if input.len().saturating_add(required) > SELECTED_INPUT_LIMIT {
            return Err(path_limit_issue());
        }
        selected_paths.insert(GitPath::from_bytes(&path));
        input.extend_from_slice(&path);
        input.push(0);
    }
    Ok((input, selected_paths))
}

fn path_limit_issue() -> GitCaptureIssue {
    GitCaptureIssue::new(
        "pathLimit",
        Some(GIT_STATUS_NAME),
        "Selected archive paths exceeded the bounded Git ignore-reconciliation count or input budget.",
    )
}

fn capture_selected_ignored(
    config: &super::GitCaptureConfig,
    input: &[u8],
    selected_paths: &HashSet<GitPath>,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<std::result::Result<Vec<GitPath>, GitCaptureIssue>> {
    if input.is_empty() {
        return Ok(Ok(Vec::new()));
    }
    let mut args = common_git_config_args();
    args.extend([
        OsString::from("check-ignore"),
        OsString::from("--stdin"),
        OsString::from("-z"),
    ]);
    let result = run_git_with_input(
        &config.executable,
        &config.repo_root,
        &args,
        Some(input.to_vec()),
        &[1],
        IGNORED_OUTPUT_LIMIT,
        config.command_timeout,
        cancel_token,
    )?;
    let output = match result {
        Ok(output) => output,
        Err(failure) => return Ok(Err(command_issue(failure))),
    };
    if output.stdout_truncated {
        return Ok(Err(GitCaptureIssue::new(
            "outputTruncated",
            Some(GIT_STATUS_NAME),
            "Selected ignored paths exceeded the bounded Git status output budget.",
        )));
    }
    if !output.stdout.is_empty() && !output.stdout.ends_with(&[0]) {
        return Ok(Err(parse_issue()));
    }

    let mut paths = HashSet::new();
    for raw_path in output.stdout.split(|byte| *byte == 0) {
        if raw_path.is_empty() {
            continue;
        }
        let path = GitPath::from_bytes(raw_path);
        if !selected_paths.contains(&path) {
            return Ok(Err(parse_issue()));
        }
        paths.insert(path);
    }
    let mut paths: Vec<_> = paths.into_iter().collect();
    paths.sort();
    Ok(Ok(paths))
}

fn command_issue(failure: GitCommandFailure) -> GitCaptureIssue {
    let (kind, detail) = match failure {
        GitCommandFailure::MissingExecutable => (
            "gitUnavailable",
            "Git became unavailable while selected ignored files were reconciled.",
        ),
        GitCommandFailure::TimedOut => (
            "commandTimeout",
            "The bounded Git ignore-reconciliation command timed out.",
        ),
        GitCommandFailure::SpawnFailed
        | GitCommandFailure::InputWriteFailed
        | GitCommandFailure::OutputReadFailed
        | GitCommandFailure::NotGitRepository
        | GitCommandFailure::NonZeroExit => (
            "commandFailure",
            "Selected ignored files could not be reconciled against Git ignore rules.",
        ),
    };
    GitCaptureIssue::new(kind, Some(GIT_STATUS_NAME), detail)
}

fn parse_issue() -> GitCaptureIssue {
    GitCaptureIssue::new(
        "statusParseFailure",
        Some(GIT_STATUS_NAME),
        "Git ignore reconciliation returned a path outside the selected archive set or malformed output.",
    )
}
