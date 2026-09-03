// Path: crates/im_host_agent/src/runtime/host_runtime/shutdown_targets.rs
// Description: The two things a host-agent shutdown must reach: the WSL backend client and the host locks

use im_agent::logging::Logger;
use im_agent::source_control::SourceControlLocks;

use crate::wsl::WslBackendClient;

use super::HostRuntime;

/// Everything a shutdown needs, cloned out under one short read lock so the
/// drain — which can last a minute or more — holds no runtime lock at all.
pub struct HostShutdownTargets {
    /// The WSL backend client only if one was ever built. A host agent that
    /// never spoke to WSL has no in-flight WSL mutation to drain: mutations can
    /// only arrive through this client.
    pub wsl_client: Option<WslBackendClient>,
    pub locks: SourceControlLocks,
    pub logger: Logger,
}

impl HostRuntime {
    pub fn shutdown_targets(&self) -> HostShutdownTargets {
        HostShutdownTargets {
            wsl_client: self.wsl_client.clone(),
            locks: self.local_backend.source_control_locks(),
            logger: self.logger.clone(),
        }
    }
}
