// Path: crates/im_host_agent/src/wsl/wsl_backend_client/hello_state.rs
// Description: Desired, applied and failed WSL clientHello state for the per-generation hello reconciler

use im_agent::error::AgentError;
use im_agent::protocol::ClientHelloCommand;

#[derive(Clone, Debug)]
pub(super) struct DesiredHello {
    pub command: ClientHelloCommand,
    pub fingerprint: String,
}

#[derive(Clone, Debug)]
struct AppliedHello {
    fingerprint: String,
    generation: u64,
    watched_repo_ids: Vec<String>,
}

#[derive(Clone, Debug)]
struct FailedHello {
    fingerprint: String,
    attempt: u64,
    code: String,
    message: String,
}

/// What a command waiting for the WSL hello should do next.
#[derive(Debug)]
pub(super) enum HelloWait {
    Ready,
    Failed(AgentError),
    Kick,
    Pending,
}

#[derive(Default, Debug)]
pub(super) struct WslHelloState {
    desired: Option<DesiredHello>,
    applied: Option<AppliedHello>,
    failure: Option<FailedHello>,
    finished_attempts: u64,
    in_flight: bool,
}

impl WslHelloState {
    /// Returns whether the desired payload changed, so the caller wakes the reconciler only then.
    pub fn set_desired(&mut self, command: Option<ClientHelloCommand>) -> Result<bool, AgentError> {
        let next = match command {
            Some(command) => Some(DesiredHello {
                fingerprint: hello_fingerprint(&command)?,
                command,
            }),
            None => None,
        };
        let changed = self.desired.as_ref().map(|hello| &hello.fingerprint)
            != next.as_ref().map(|hello| &hello.fingerprint);
        self.desired = next;
        Ok(changed)
    }

    pub fn has_desired(&self) -> bool {
        self.desired.is_some()
    }

    pub fn finished_attempts(&self) -> u64 {
        self.finished_attempts
    }

    /// Claims the next attempt, or `None` when nothing is desired or it is applied for `generation`.
    pub fn begin_attempt(&mut self, generation: u64) -> Option<DesiredHello> {
        if self.in_flight || self.is_applied(generation) {
            return None;
        }
        let desired = self.desired.clone()?;
        self.in_flight = true;
        Some(desired)
    }

    /// Records what WSL now runs; true when this is the first apply of that payload for the generation.
    pub fn finish_applied(
        &mut self,
        fingerprint: &str,
        generation: u64,
        watched: Vec<String>,
    ) -> bool {
        self.in_flight = false;
        self.finished_attempts += 1;
        let repeat = self.applied.as_ref().is_some_and(|applied| {
            applied.fingerprint == fingerprint && applied.generation == generation
        });
        self.applied = Some(AppliedHello {
            fingerprint: fingerprint.to_string(),
            generation,
            watched_repo_ids: watched,
        });
        self.failure = None;
        !repeat
    }

    pub fn finish_failed(&mut self, fingerprint: &str, err: &AgentError) {
        self.in_flight = false;
        self.finished_attempts += 1;
        self.failure = Some(FailedHello {
            fingerprint: fingerprint.to_string(),
            attempt: self.finished_attempts,
            code: err.code().to_string(),
            message: err.message().to_string(),
        });
    }

    /// A failure counts for a waiter only if it finished after the waiter began (`seen_attempts`).
    pub fn wait_status(&self, generation: u64, seen_attempts: u64) -> HelloWait {
        if self.desired.is_none() || self.is_applied(generation) {
            return HelloWait::Ready;
        }
        if self.in_flight {
            return HelloWait::Pending;
        }
        let fingerprint = self
            .desired
            .as_ref()
            .map(|hello| hello.fingerprint.as_str());
        match &self.failure {
            Some(failure)
                if failure.attempt > seen_attempts
                    && Some(failure.fingerprint.as_str()) == fingerprint =>
            {
                HelloWait::Failed(AgentError::new(
                    failure.code.clone(),
                    failure.message.clone(),
                ))
            }
            _ => HelloWait::Kick,
        }
    }

    /// Watched ids WSL reported for `generation`; empty when nothing is applied on it.
    pub fn applied_watched_repo_ids(&self, generation: u64) -> Vec<String> {
        match &self.applied {
            Some(applied) if applied.generation == generation => applied.watched_repo_ids.clone(),
            _ => Vec::new(),
        }
    }

    fn is_applied(&self, generation: u64) -> bool {
        match (&self.desired, &self.applied) {
            (Some(desired), Some(applied)) => {
                desired.fingerprint == applied.fingerprint && applied.generation == generation
            }
            _ => false,
        }
    }
}

fn hello_fingerprint(command: &ClientHelloCommand) -> Result<String, AgentError> {
    serde_json::to_string(command).map_err(|err| AgentError::new("INVALID_CONFIG", err.to_string()))
}

#[cfg(test)]
#[path = "hello_state_tests.rs"]
mod tests;
