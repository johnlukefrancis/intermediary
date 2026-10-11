// Path: crates/im_agent/src/server/socket_writer.rs
// Description: The per-connection socket writer shared by both agents - responses first, then relay events, closing the socket when the relay ends

use futures_util::{Sink, SinkExt};
use serde_json::json;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use crate::logging::Logger;

use super::EventRelay;

/// Why the writer stopped; the connection's read loop ends on either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriterEnd {
    /// The relay ended (it lagged and lost events): the client was sent a close so it resyncs.
    RelayClosed,
    /// The response channel closed or a send failed.
    Socket,
}

/// The close a lagged client receives: 1013 tells it to reconnect, which re-runs its hello.
pub const RELAY_LAGGED_CLOSE_REASON: &str = "event relay lagged; reconnect to resync";

pub fn spawn_socket_writer<S>(
    mut sink: S,
    mut response_rx: mpsc::UnboundedReceiver<Message>,
    mut event_rx: EventRelay,
    logger: Logger,
    peer: String,
) -> JoinHandle<WriterEnd>
where
    S: Sink<Message, Error = WsError> + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        loop {
            let next = tokio::select! {
                biased;
                response = response_rx.recv() => match response {
                    Some(message) => message,
                    None => return WriterEnd::Socket,
                },
                event = event_rx.recv() => match event {
                    Some(text) => Message::Text(text),
                    None => break,
                },
            };
            if let Err(err) = sink.send(next).await {
                logger.warn(
                    "Failed to send WebSocket message",
                    Some(json!({"peer": peer, "error": err.to_string()})),
                );
                return WriterEnd::Socket;
            }
        }
        let close = Message::Close(Some(CloseFrame {
            code: CloseCode::Again,
            reason: RELAY_LAGGED_CLOSE_REASON.into(),
        }));
        let _ = sink.send(close).await;
        logger.warn(
            "Event relay ended; closed the connection so the client resyncs",
            Some(json!({"peer": peer})),
        );
        WriterEnd::RelayClosed
    })
}

#[cfg(test)]
#[path = "socket_writer_tests.rs"]
mod tests;
