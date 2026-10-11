// Path: crates/im_host_agent/src/server/request_dispatch.rs
// Description: Connection-scoped request dispatch and cancellation of the current hover inspection

use im_agent::bundles::BundleCancelToken;
use im_agent::error::to_response_error;
use im_agent::protocol::{InboundRequestEnvelope, ResponseEnvelope, UiCommand};
use serde_json::json;
use tokio::sync::mpsc;
use tokio::task::AbortHandle;
use tokio_tungstenite::tungstenite::Message;

use super::connection::ConnectionContext;
use super::dispatch;

#[derive(Default)]
pub(super) struct ConnectionRequests {
    inspection: Option<(String, BundleCancelToken, AbortHandle)>,
}

impl ConnectionRequests {
    pub(super) fn receive(
        &mut self,
        raw: &str,
        ctx: &ConnectionContext,
        responses: &mpsc::UnboundedSender<Message>,
    ) {
        let envelope: InboundRequestEnvelope = match serde_json::from_str(raw) {
            Ok(envelope) => envelope,
            Err(error) => {
                ctx.logger.warn(
                    "Invalid JSON message",
                    Some(json!({"error": error.to_string()})),
                );
                return;
            }
        };
        let (request_id, command) = match envelope {
            InboundRequestEnvelope::Request {
                request_id,
                payload,
            } => (request_id, *payload),
            InboundRequestEnvelope::Cancel { request_id } => {
                if self
                    .inspection
                    .as_ref()
                    .is_some_and(|(id, _, _)| id == &request_id)
                {
                    self.cancel();
                }
                return;
            }
        };
        let cancel = if matches!(command, UiCommand::InspectBundleEntry(_)) {
            self.cancel();
            Some(BundleCancelToken::new())
        } else {
            None
        };
        let request_cancel = cancel.clone();
        let response_id = request_id.clone();
        let ctx = ctx.clone();
        let responses = responses.clone();
        let task = tokio::spawn(async move {
            let response = match dispatch::dispatch_command(command, &ctx, request_cancel).await {
                Ok(payload) => ResponseEnvelope::ok(response_id, payload),
                Err(error) => ResponseEnvelope::error(response_id, to_response_error(&error)),
            };
            match serde_json::to_string(&response) {
                Ok(text) => {
                    let _ = responses.send(Message::Text(text));
                }
                Err(error) => ctx.logger.error(
                    "Failed to serialize response",
                    Some(json!({"error": error.to_string()})),
                ),
            }
        });
        if let Some(cancel) = cancel {
            self.inspection = Some((request_id, cancel, task.abort_handle()));
        }
    }

    fn cancel(&mut self) {
        if let Some((_, cancel, task)) = self.inspection.take() {
            cancel.cancel();
            task.abort();
        }
    }
}

impl Drop for ConnectionRequests {
    fn drop(&mut self) {
        self.cancel();
    }
}
