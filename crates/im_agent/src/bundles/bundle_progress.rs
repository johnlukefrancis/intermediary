// Path: crates/im_agent/src/bundles/bundle_progress.rs
// Description: Bundle progress forwarding from im_bundle to agent events

use tokio::sync::mpsc;

use im_bundle::progress::ProgressMessage;

use crate::protocol::{AgentEvent, BundleBuildProgressEvent};
use crate::server::EventBus;

pub(crate) fn start_progress_forwarder(
    event_bus: &EventBus,
    repo_id: String,
    preset_id: String,
) -> (
    mpsc::UnboundedSender<ProgressMessage>,
    tokio::task::JoinHandle<()>,
) {
    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<ProgressMessage>();
    let progress_bus = event_bus.clone();

    let handle = tokio::spawn(async move {
        while let Some(message) = progress_rx.recv().await {
            if let Some(event) = map_progress_event(&repo_id, &preset_id, message) {
                progress_bus.broadcast_event(AgentEvent::BundleBuildProgress(event));
            }
        }
    });

    (progress_tx, handle)
}

fn map_progress_event(
    repo_id: &str,
    preset_id: &str,
    message: ProgressMessage,
) -> Option<BundleBuildProgressEvent> {
    match message {
        ProgressMessage::Progress {
            phase,
            files_done,
            files_total,
            current_file,
            current_bytes_done,
            current_bytes_total,
            bytes_done_total_best_effort,
        } => Some(BundleBuildProgressEvent {
            repo_id: repo_id.to_string(),
            preset_id: preset_id.to_string(),
            phase: phase.to_string(),
            files_done,
            files_total,
            current_file,
            current_bytes_done,
            current_bytes_total,
            bytes_done_total_best_effort,
        }),
        ProgressMessage::Done { .. } => None,
    }
}
