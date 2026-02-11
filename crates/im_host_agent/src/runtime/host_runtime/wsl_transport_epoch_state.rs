// Path: crates/im_host_agent/src/runtime/host_runtime/wsl_transport_epoch_state.rs
// Description: Tracks WSL transport error emission by backend connection generation for de-noised offline transitions

#[derive(Debug, Default)]
pub(super) struct WslTransportEpochState {
    offline_error_generation: Option<u64>,
}

impl WslTransportEpochState {
    pub fn mark_success(&mut self, generation: u64) -> bool {
        if self.offline_error_generation == Some(generation) {
            self.offline_error_generation = None;
            return true;
        }

        false
    }

    pub fn should_emit_offline_error(&mut self, generation: u64) -> bool {
        if self.offline_error_generation == Some(generation) {
            return false;
        }

        self.offline_error_generation = Some(generation);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::WslTransportEpochState;

    #[test]
    fn suppresses_duplicate_offline_error_within_same_generation() {
        let mut state = WslTransportEpochState::default();
        assert!(state.should_emit_offline_error(2));
        assert!(!state.should_emit_offline_error(2));
    }

    #[test]
    fn allows_new_offline_error_after_success() {
        let mut state = WslTransportEpochState::default();
        assert!(state.should_emit_offline_error(3));
        assert!(state.mark_success(3));
        assert!(state.should_emit_offline_error(3));
    }

    #[test]
    fn allows_new_offline_error_for_new_generation() {
        let mut state = WslTransportEpochState::default();
        assert!(state.should_emit_offline_error(4));
        assert!(state.should_emit_offline_error(5));
    }

    #[test]
    fn success_reports_recovery_once_per_offline_emission() {
        let mut state = WslTransportEpochState::default();
        assert!(state.should_emit_offline_error(9));
        assert!(state.mark_success(9));
        assert!(!state.mark_success(9));
    }

    #[test]
    fn success_does_not_recover_for_different_generation() {
        let mut state = WslTransportEpochState::default();
        assert!(state.should_emit_offline_error(10));
        assert!(!state.mark_success(11));
    }
}
