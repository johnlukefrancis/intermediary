// Path: crates/im_host_agent/src/runtime/repo_backend.rs
// Description: Repo backend kind mapping for host-agent routing

use im_agent::runtime::RepoConfig;
use im_agent::runtime::RepoRootKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoBackend {
    Host,
    Wsl,
}

impl RepoBackend {
    pub fn from_repo_config(repo: &RepoConfig) -> Self {
        match repo.root_kind() {
            RepoRootKind::Wsl => Self::Wsl,
            RepoRootKind::Host => Self::Host,
        }
    }
}
