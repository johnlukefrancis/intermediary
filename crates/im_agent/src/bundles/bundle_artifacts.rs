// Path: crates/im_agent/src/bundles/bundle_artifacts.rs
// Description: Repo-declared bundle artifact hook execution and status injection

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use chrono::Utc;
use im_bundle::plan::BundleExtraEntry;
use serde::{Deserialize, Serialize};

use crate::error::AgentError;

const CONFIG_PATH: &str = ".intermediary/bundle_artifacts.json";
const DEFAULT_STATUS_ARCHIVE_PATH: &str = "AGENT_HANDOFF/INTERMEDIARY_BUNDLE_ARTIFACTS_STATUS.json";
const DEFAULT_TIMEOUT_MS: u64 = 15_000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundleArtifactConfig {
    schema: String,
    #[serde(default)]
    status_archive_path: Option<String>,
    #[serde(default)]
    artifacts: Vec<BundleArtifactSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundleArtifactSpec {
    id: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    timeout_ms: Option<u64>,
    #[serde(default)]
    command: Option<Vec<String>>,
    #[serde(default)]
    outputs: Vec<BundleArtifactOutput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundleArtifactOutput {
    path: String,
    archive_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BundleArtifactStatusFile {
    schema: String,
    generated_at: String,
    config_path: String,
    artifacts: Vec<BundleArtifactStatus>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BundleArtifactStatus {
    id: String,
    command_status: String,
    exit_code: Option<i32>,
    timed_out: bool,
    required: bool,
    outputs: Vec<BundleArtifactOutputStatus>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BundleArtifactOutputStatus {
    archive_path: String,
    source_path: String,
    present: bool,
}

pub(crate) fn collect_bundle_artifacts(
    repo_root: &Path,
    artifact_dir: &Path,
) -> Result<Vec<BundleExtraEntry>, AgentError> {
    let config_path = repo_root.join(CONFIG_PATH);
    if !config_path.exists() {
        return Ok(vec![]);
    }

    std::fs::create_dir_all(artifact_dir).map_err(|err| {
        AgentError::internal(format!("Failed to create bundle artifact directory: {err}"))
    })?;

    let config_text = std::fs::read_to_string(&config_path).map_err(|err| {
        AgentError::internal(format!("Failed to read bundle artifact config: {err}"))
    })?;
    let config: BundleArtifactConfig = serde_json::from_str(&config_text).map_err(|err| {
        AgentError::internal(format!("Failed to parse bundle artifact config: {err}"))
    })?;

    if config.schema != "intermediary-bundle-artifacts-v1" {
        return Err(AgentError::internal(format!(
            "Unsupported bundle artifact schema: {}",
            config.schema
        )));
    }

    let mut extra_entries = Vec::new();
    let mut statuses = Vec::new();
    let allowed_roots = ArtifactAllowedRoots::canonicalize(repo_root, artifact_dir)?;

    for artifact in &config.artifacts {
        let command_result = run_artifact_command(repo_root, artifact_dir, artifact)?;
        if artifact.required && command_result.status != "ok" {
            return Err(AgentError::internal(format!(
                "Required bundle artifact command failed for {}: status={} exitCode={:?}",
                artifact.id, command_result.status, command_result.exit_code
            )));
        }
        let mut outputs = Vec::new();

        for output in &artifact.outputs {
            let source_path = expand_path(&output.path, repo_root, artifact_dir);
            let resolved_source_path = resolve_artifact_output_path(&source_path, &allowed_roots)?;
            let present = resolved_source_path.is_file();
            outputs.push(BundleArtifactOutputStatus {
                archive_path: output.archive_path.clone(),
                source_path: resolved_source_path.to_string_lossy().to_string(),
                present,
            });
            if present {
                extra_entries.push(BundleExtraEntry {
                    source_path: resolved_source_path,
                    archive_path: output.archive_path.clone(),
                });
            } else if artifact.required {
                return Err(AgentError::internal(format!(
                    "Required bundle artifact output missing for {}: {}",
                    artifact.id, output.path
                )));
            }
        }

        statuses.push(BundleArtifactStatus {
            id: artifact.id.clone(),
            command_status: command_result.status,
            exit_code: command_result.exit_code,
            timed_out: command_result.timed_out,
            required: artifact.required,
            outputs,
        });
    }

    let status_path = artifact_dir.join("INTERMEDIARY_BUNDLE_ARTIFACTS_STATUS.json");
    let status_file = BundleArtifactStatusFile {
        schema: "intermediary-bundle-artifacts-status-v1".to_string(),
        generated_at: Utc::now().to_rfc3339(),
        config_path: CONFIG_PATH.to_string(),
        artifacts: statuses,
    };
    let status_json = serde_json::to_string_pretty(&status_file).map_err(|err| {
        AgentError::internal(format!("Failed to serialize bundle artifact status: {err}"))
    })?;
    std::fs::write(&status_path, format!("{status_json}\n")).map_err(|err| {
        AgentError::internal(format!("Failed to write bundle artifact status: {err}"))
    })?;
    extra_entries.push(BundleExtraEntry {
        source_path: status_path,
        archive_path: config
            .status_archive_path
            .unwrap_or_else(|| DEFAULT_STATUS_ARCHIVE_PATH.to_string()),
    });

    Ok(extra_entries)
}

struct ArtifactAllowedRoots {
    repo_root: PathBuf,
    artifact_dir: PathBuf,
}

impl ArtifactAllowedRoots {
    fn canonicalize(repo_root: &Path, artifact_dir: &Path) -> Result<Self, AgentError> {
        let repo_root = repo_root.canonicalize().map_err(|err| {
            AgentError::internal(format!("Failed to canonicalize repo root: {err}"))
        })?;
        let artifact_dir = artifact_dir.canonicalize().map_err(|err| {
            AgentError::internal(format!("Failed to canonicalize artifact directory: {err}"))
        })?;
        Ok(Self {
            repo_root,
            artifact_dir,
        })
    }

    fn contains(&self, path: &Path) -> bool {
        path.starts_with(&self.repo_root) || path.starts_with(&self.artifact_dir)
    }
}

struct ArtifactCommandResult {
    status: String,
    exit_code: Option<i32>,
    timed_out: bool,
}

fn run_artifact_command(
    repo_root: &Path,
    artifact_dir: &Path,
    artifact: &BundleArtifactSpec,
) -> Result<ArtifactCommandResult, AgentError> {
    let Some(command) = artifact.command.as_ref() else {
        return Ok(ArtifactCommandResult {
            status: "not-configured".to_string(),
            exit_code: None,
            timed_out: false,
        });
    };
    if command.is_empty() {
        return Err(AgentError::internal(format!(
            "Bundle artifact command is empty for {}",
            artifact.id
        )));
    }

    let program = expand_string(&command[0], repo_root, artifact_dir);
    let args: Vec<String> = command[1..]
        .iter()
        .map(|arg| expand_string(arg, repo_root, artifact_dir))
        .collect();

    let mut child = Command::new(program)
        .args(args)
        .current_dir(repo_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| {
            AgentError::internal(format!(
                "Failed to start bundle artifact command for {}: {err}",
                artifact.id
            ))
        })?;

    let timeout = Duration::from_millis(artifact.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS));
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|err| {
            AgentError::internal(format!(
                "Failed to poll bundle artifact command for {}: {err}",
                artifact.id
            ))
        })? {
            return Ok(ArtifactCommandResult {
                status: if status.success() { "ok" } else { "failed" }.to_string(),
                exit_code: status.code(),
                timed_out: false,
            });
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            if artifact.required {
                return Err(AgentError::internal(format!(
                    "Required bundle artifact command timed out for {}",
                    artifact.id
                )));
            }
            return Ok(ArtifactCommandResult {
                status: "timed-out".to_string(),
                exit_code: None,
                timed_out: true,
            });
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn expand_path(value: &str, repo_root: &Path, artifact_dir: &Path) -> PathBuf {
    PathBuf::from(expand_string(value, repo_root, artifact_dir))
}

fn expand_string(value: &str, repo_root: &Path, artifact_dir: &Path) -> String {
    value
        .replace("{repoRoot}", &repo_root.to_string_lossy())
        .replace("{artifactDir}", &artifact_dir.to_string_lossy())
}

fn resolve_artifact_output_path(
    source_path: &Path,
    allowed_roots: &ArtifactAllowedRoots,
) -> Result<PathBuf, AgentError> {
    if !source_path.exists() {
        return Ok(source_path.to_path_buf());
    }
    let canonical = source_path.canonicalize().map_err(|err| {
        AgentError::internal(format!(
            "Failed to canonicalize bundle artifact output {}: {err}",
            source_path.display()
        ))
    })?;
    if !allowed_roots.contains(&canonical) {
        return Err(AgentError::internal(format!(
            "Bundle artifact output is outside approved roots: {}",
            canonical.display()
        )));
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn copies_declared_outputs_and_writes_status() {
        let dir = tempdir().unwrap();
        let repo_root = dir.path().join("repo");
        let artifact_dir = dir.path().join("artifacts");
        std::fs::create_dir_all(repo_root.join(".intermediary")).unwrap();
        std::fs::write(repo_root.join("handoff.json"), "{\"ok\":true}").unwrap();
        std::fs::write(
            repo_root.join(CONFIG_PATH),
            r#"{
              "schema": "intermediary-bundle-artifacts-v1",
              "artifacts": [{
                "id": "existing-file",
                "outputs": [{
                  "path": "{repoRoot}/handoff.json",
                  "archivePath": "AGENT_HANDOFF/handoff.json"
                }]
              }]
            }"#,
        )
        .unwrap();

        let entries = collect_bundle_artifacts(&repo_root, &artifact_dir).unwrap();
        assert!(entries
            .iter()
            .any(|entry| entry.archive_path == "AGENT_HANDOFF/handoff.json"));
        assert!(entries.iter().any(|entry| {
            entry.archive_path == "AGENT_HANDOFF/INTERMEDIARY_BUNDLE_ARTIFACTS_STATUS.json"
        }));
    }

    #[test]
    fn rejects_failed_required_command_even_when_output_exists() {
        let dir = tempdir().unwrap();
        let repo_root = dir.path().join("repo");
        let artifact_dir = dir.path().join("artifacts");
        std::fs::create_dir_all(repo_root.join(".intermediary")).unwrap();
        std::fs::create_dir_all(&artifact_dir).unwrap();
        std::fs::write(artifact_dir.join("stale.json"), "{\"stale\":true}").unwrap();
        std::fs::write(
            repo_root.join(CONFIG_PATH),
            r#"{
              "schema": "intermediary-bundle-artifacts-v1",
              "artifacts": [{
                "id": "required-stale-output",
                "required": true,
                "command": ["false"],
                "outputs": [{
                  "path": "{artifactDir}/stale.json",
                  "archivePath": "AGENT_HANDOFF/stale.json"
                }]
              }]
            }"#,
        )
        .unwrap();

        let err = collect_bundle_artifacts(&repo_root, &artifact_dir).unwrap_err();
        assert_eq!(err.code(), "INTERNAL_ERROR");
        assert!(err
            .message()
            .contains("Required bundle artifact command failed"));
    }

    #[test]
    fn rejects_declared_outputs_outside_repo_or_artifact_dir() {
        let dir = tempdir().unwrap();
        let repo_root = dir.path().join("repo");
        let sibling_dir = dir.path().join("sibling");
        let artifact_dir = dir.path().join("artifacts");
        std::fs::create_dir_all(repo_root.join(".intermediary")).unwrap();
        std::fs::create_dir_all(&sibling_dir).unwrap();
        std::fs::write(sibling_dir.join("secret.json"), "{\"secret\":true}").unwrap();
        std::fs::write(
            repo_root.join(CONFIG_PATH),
            r#"{
              "schema": "intermediary-bundle-artifacts-v1",
              "artifacts": [{
                "id": "escape-output",
                "outputs": [{
                  "path": "{repoRoot}/../sibling/secret.json",
                  "archivePath": "AGENT_HANDOFF/secret.json"
                }]
              }]
            }"#,
        )
        .unwrap();

        let err = collect_bundle_artifacts(&repo_root, &artifact_dir).unwrap_err();
        assert_eq!(err.code(), "INTERNAL_ERROR");
        assert!(err.message().contains("outside approved roots"));
    }
}
