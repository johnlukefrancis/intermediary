// Path: crates/im_bundle/src/git_capture/verification.rs
// Description: Streaming selected-file coherence verification for Git bundle capture

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use crate::cancel::{check_cancelled, BundleCancelToken};
use crate::error::Result;

use super::{GitCaptureIssue, GIT_DIFF_NAME};

const VERIFY_TIMEOUT: Duration = Duration::from_secs(15);
const VERIFY_BUFFER_SIZE: usize = 256 * 1024;

pub(crate) type WrittenEntryDigests = HashMap<PathBuf, [u8; 32]>;

pub(crate) struct VerificationResult {
    pub(crate) drifted: bool,
    pub(crate) issues: Vec<GitCaptureIssue>,
}

pub(crate) struct DigestCapture {
    pub(crate) digests: WrittenEntryDigests,
    pub(crate) complete: bool,
    pub(crate) timed_out: bool,
}

pub(crate) fn capture_current_digests(
    repo_root: &Path,
    watched_paths: &HashSet<PathBuf>,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<DigestCapture> {
    let started = Instant::now();
    let mut buffer = vec![0u8; VERIFY_BUFFER_SIZE];
    let mut digests = WrittenEntryDigests::new();
    let mut complete = true;
    let mut timed_out = false;

    for path in watched_paths {
        check_cancelled(cancel_token)?;
        if started.elapsed() >= VERIFY_TIMEOUT {
            timed_out = true;
            complete = false;
            break;
        }
        let file = match File::open(repo_root.join(path)) {
            Ok(file) => file,
            Err(_) => {
                complete = false;
                continue;
            }
        };
        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        loop {
            check_cancelled(cancel_token)?;
            if started.elapsed() >= VERIFY_TIMEOUT {
                timed_out = true;
                complete = false;
                break;
            }
            let read = match reader.read(&mut buffer) {
                Ok(read) => read,
                Err(_) => {
                    complete = false;
                    break;
                }
            };
            if read == 0 {
                digests.insert(path.clone(), hasher.finalize().into());
                break;
            }
            hasher.update(&buffer[..read]);
        }
        if timed_out {
            break;
        }
    }
    Ok(DigestCapture {
        digests,
        complete,
        timed_out,
    })
}

pub(crate) fn verify_written_state(
    repo_root: &Path,
    watched_paths: &HashSet<PathBuf>,
    written_digests: &WrittenEntryDigests,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<VerificationResult> {
    let captured = capture_current_digests(repo_root, watched_paths, cancel_token)?;
    let drifted = !captured.complete
        || watched_paths
            .iter()
            .any(|path| captured.digests.get(path) != written_digests.get(path));
    let timed_out = captured.timed_out;

    let mut issues = Vec::new();
    if drifted {
        issues.push(GitCaptureIssue::new(
            "captureDrift",
            Some(GIT_DIFF_NAME),
            "At least one selected file differs from the bytes written into the archive.",
        ));
    }
    if timed_out {
        issues.push(GitCaptureIssue::new(
            "verificationTimeout",
            Some(GIT_DIFF_NAME),
            "Selected-file coherence verification reached its bounded time budget.",
        ));
    }
    Ok(VerificationResult { drifted, issues })
}
