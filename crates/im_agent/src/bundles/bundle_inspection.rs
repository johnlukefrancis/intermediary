// Path: crates/im_agent/src/bundles/bundle_inspection.rs
// Description: Owned blocking inspection worker shared by host and WSL repository routes

use std::path::Path;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::Semaphore;

use crate::error::AgentError;
use crate::protocol::{InspectBundleEntryCommand, InspectBundleEntryResult};
use im_bundle::cancel::BundleCancelToken;

pub async fn inspect_bundle_entry(
    repo_root: String,
    command: InspectBundleEntryCommand,
    cancel: BundleCancelToken,
) -> Result<InspectBundleEntryResult, AgentError> {
    static WORKERS: OnceLock<Arc<Semaphore>> = OnceLock::new();
    let permit = WORKERS
        .get_or_init(|| Arc::new(Semaphore::new(2)))
        .clone()
        .try_acquire_owned()
        .map_err(|_| {
            AgentError::new(
                "INSPECTION_BUSY",
                "A folder measurement is still finishing; hover again",
            )
        })?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let inspection = im_bundle::inspection::inspect_entry(
            Path::new(&repo_root),
            &command.path,
            &command.selection,
            &command.global_excludes,
            &cancel,
        )
        .map_err(|error| AgentError::new("INSPECTION_FAILED", error.to_string()))?;
        Ok(InspectBundleEntryResult {
            repo_id: command.repo_id,
            path: command.path,
            inspection,
        })
    })
    .await
    .map_err(|error| AgentError::internal(format!("Folder measurement failed: {error}")))?
}
