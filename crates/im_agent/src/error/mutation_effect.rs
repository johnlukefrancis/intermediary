// Path: crates/im_agent/src/error/mutation_effect.rs
// Description: Outcome certainty (`details.effect`) carried by every source-control mutation error

use serde_json::{Map, Value};

use super::AgentError;

/// What a failed mutation did to the repository. `NotApplied` is a proof: the
/// agent knows the request never crossed its effect boundary (pre-flight
/// refusal, a Git process that never started, a non-zero exit from an atomic
/// index command). `Unknown` is everything else — a timeout, a forced stop, a
/// failed follow-up read — where only a fresh read can decide.
///
/// The error namespace never carries this meaning: `GIT_*` says which layer
/// spoke, not whether the repository changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationEffect {
    NotApplied,
    Unknown,
}

impl MutationEffect {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotApplied => "notApplied",
            Self::Unknown => "unknown",
        }
    }
}

impl AgentError {
    /// Records the proven effect, replacing any effect already recorded.
    pub fn with_effect(self, effect: MutationEffect) -> Self {
        set_effect(self, effect)
    }

    /// Records `effect` only where no site has proven one yet, so the outer
    /// mutation boundary can guarantee the field exists without overwriting a
    /// proof made closer to the Git process.
    pub fn with_default_effect(self, effect: MutationEffect) -> Self {
        if self.effect().is_some() {
            return self;
        }
        set_effect(self, effect)
    }

    /// The recorded effect string, when one was recorded.
    pub fn effect(&self) -> Option<&str> {
        self.details()?.get("effect")?.as_str()
    }
}

fn set_effect(error: AgentError, effect: MutationEffect) -> AgentError {
    let mut details = match error.details().cloned() {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    };
    details.insert("effect".to_string(), Value::from(effect.as_str()));
    error.with_details(Value::Object(details))
}

#[cfg(test)]
mod tests {
    use super::{AgentError, MutationEffect};

    #[test]
    fn effect_is_added_beside_existing_details() {
        let error = AgentError::new("X", "x")
            .with_details(serde_json::json!({ "kind": "commit" }))
            .with_effect(MutationEffect::Unknown);
        assert_eq!(
            error.details(),
            Some(&serde_json::json!({ "kind": "commit", "effect": "unknown" }))
        );
    }

    #[test]
    fn a_proven_effect_survives_the_outer_default() {
        let error = AgentError::new("X", "x")
            .with_effect(MutationEffect::NotApplied)
            .with_default_effect(MutationEffect::Unknown);
        assert_eq!(error.effect(), Some("notApplied"));
    }

    #[test]
    fn the_outer_default_fills_an_unclassified_failure() {
        let error = AgentError::new("X", "x").with_default_effect(MutationEffect::Unknown);
        assert_eq!(error.effect(), Some("unknown"));
    }
}
