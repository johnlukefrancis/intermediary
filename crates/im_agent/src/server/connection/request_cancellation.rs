// Path: crates/im_agent/src/server/connection/request_cancellation.rs
// Description: Cooperative cancellation handles for active backend requests

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use im_bundle::cancel::BundleCancelToken;

use crate::error::AgentError;
use crate::protocol::UiCommand;
use crate::staging::StageFileCancelToken;

#[derive(Clone)]
pub(super) enum RequestCancellation {
    Bundle(BundleCancelToken),
    StageFile(StageFileCancelToken),
    Passive(Arc<AtomicBool>),
}

impl RequestCancellation {
    pub(super) fn for_command(command: &UiCommand) -> Self {
        match command {
            UiCommand::BuildBundle(_) => Self::Bundle(BundleCancelToken::new()),
            UiCommand::StageFile(_) => Self::StageFile(StageFileCancelToken::new()),
            _ => Self::Passive(Arc::new(AtomicBool::new(false))),
        }
    }

    pub(super) fn cancel(&self) {
        match self {
            Self::Bundle(token) => token.cancel(),
            Self::StageFile(token) => token.cancel(),
            Self::Passive(cancelled) => cancelled.store(true, Ordering::SeqCst),
        }
    }

    pub(super) fn is_cancelled(&self) -> bool {
        match self {
            Self::Bundle(token) => token.is_cancelled(),
            Self::StageFile(token) => token.is_cancelled(),
            Self::Passive(cancelled) => cancelled.load(Ordering::SeqCst),
        }
    }

    pub(super) fn bundle_token(&self) -> Result<BundleCancelToken, AgentError> {
        match self {
            Self::Bundle(token) => Ok(token.clone()),
            _ => Err(AgentError::internal(
                "Bundle request is missing its cancellation token",
            )),
        }
    }

    pub(super) fn stage_file_token(&self) -> Result<StageFileCancelToken, AgentError> {
        match self {
            Self::StageFile(token) => Ok(token.clone()),
            _ => Err(AgentError::internal(
                "Stage-file request is missing its cancellation token",
            )),
        }
    }
}
