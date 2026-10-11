// Path: crates/im_host_agent/src/runtime/host_runtime/bundle_forwarding.rs
// Description: Host build-bundle preparation and cancellation helpers for HostRuntime

use im_agent::bundles::BuildBundleOptions;
use im_agent::error::AgentError;
use im_agent::protocol::{BuildBundleCommand, CancelBundleBuildCommand, UiResponse};

use super::HostRuntime;

impl HostRuntime {
    /// Resolves a host build under the lock; the build itself runs after the lock is released.
    pub fn prepare_host_build_bundle(
        &self,
        command: BuildBundleCommand,
    ) -> Result<BuildBundleOptions, AgentError> {
        self.local_backend.prepare_build_bundle(command)
    }

    pub fn dispatch_host_cancel_bundle_build(
        &self,
        command: CancelBundleBuildCommand,
    ) -> UiResponse {
        UiResponse::CancelBundleBuildResult(self.local_backend.cancel_bundle_build(command))
    }
}
