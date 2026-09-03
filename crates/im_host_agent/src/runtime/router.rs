// Path: crates/im_host_agent/src/runtime/router.rs
// Description: Repo-id command routing for host-agent backend selection

use std::collections::HashMap;

use im_agent::error::AgentError;
use im_agent::protocol::UiCommand;

use super::RepoBackend;

pub fn resolve_repo_backend(
    command: &UiCommand,
    repo_backends: &HashMap<String, RepoBackend>,
) -> Result<Option<RepoBackend>, AgentError> {
    let Some(repo_id) = command.repo_id() else {
        return Ok(None);
    };

    let backend = repo_backends
        .get(repo_id)
        .copied()
        .ok_or_else(|| AgentError::new("UNKNOWN_REPO", format!("Unknown repo: {repo_id}")))?;

    Ok(Some(backend))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use im_agent::protocol::{UiCommand, WatchRepoCommand};

    use super::*;

    #[test]
    fn routes_host_repo_command_to_host_backend() {
        let mut routes = HashMap::new();
        routes.insert("repo_host".to_string(), RepoBackend::Host);

        let command = UiCommand::WatchRepo(WatchRepoCommand {
            repo_id: "repo_host".to_string(),
        });

        let routed = resolve_repo_backend(&command, &routes).expect("route");
        assert_eq!(routed, Some(RepoBackend::Host));
    }

    #[test]
    fn routes_wsl_repo_command_to_wsl_backend() {
        let mut routes = HashMap::new();
        routes.insert("repo_wsl".to_string(), RepoBackend::Wsl);

        let command = UiCommand::WatchRepo(WatchRepoCommand {
            repo_id: "repo_wsl".to_string(),
        });

        let routed = resolve_repo_backend(&command, &routes).expect("route");
        assert_eq!(routed, Some(RepoBackend::Wsl));
    }
}
