// Path: crates/im_agent/src/bundles/git_info.rs
// Description: Best-effort git info lookup for bundle manifests

use std::time::Duration;

use tokio::process::Command;
use tokio::time::timeout;

use crate::logging::Logger;

const GIT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct GitInfo {
    pub head_sha: Option<String>,
    pub short_sha: Option<String>,
    pub branch: Option<String>,
}

pub async fn get_git_info(repo_root: &str, logger: &Logger) -> GitInfo {
    let head_sha = run_git(repo_root, logger, &["rev-parse", "HEAD"]).await;
    let short_sha = head_sha
        .as_ref()
        .and_then(|sha| sha.get(0..7))
        .map(|slice| slice.to_string());
    let branch = run_git(repo_root, logger, &["rev-parse", "--abbrev-ref", "HEAD"]).await;

    GitInfo {
        head_sha,
        short_sha,
        branch,
    }
}

async fn run_git(repo_root: &str, logger: &Logger, args: &[&str]) -> Option<String> {
    let mut command = Command::new("git");
    command.args(args).current_dir(repo_root);

    let output = match timeout(GIT_TIMEOUT, command.output()).await {
        Ok(result) => result,
        Err(_) => {
            logger.debug(
                "Git command timed out",
                Some(serde_json::json!({"repoRoot": repo_root, "args": args})),
            );
            return None;
        }
    };

    let output = match output {
        Ok(output) => output,
        Err(err) => {
            logger.debug(
                "Git command failed",
                Some(serde_json::json!({"repoRoot": repo_root, "args": args, "error": err.to_string()})),
            );
            return None;
        }
    };

    if !output.status.success() {
        logger.debug(
            "Git command returned non-zero status",
            Some(serde_json::json!({
                "repoRoot": repo_root,
                "args": args,
                "status": output.status.code(),
            })),
        );
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}
