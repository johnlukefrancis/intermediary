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
    let repo_id = repo_id_for_command(command);
    let Some(repo_id) = repo_id else {
        return Ok(None);
    };

    let backend = repo_backends
        .get(repo_id)
        .copied()
        .ok_or_else(|| AgentError::new("UNKNOWN_REPO", format!("Unknown repo: {repo_id}")))?;

    Ok(Some(backend))
}

fn repo_id_for_command(command: &UiCommand) -> Option<&str> {
    match command {
        UiCommand::WatchRepo(command) => Some(&command.repo_id),
        UiCommand::Refresh(command) => Some(&command.repo_id),
        UiCommand::StageFile(command) => Some(&command.repo_id),
        UiCommand::BuildBundle(command) => Some(&command.repo_id),
        UiCommand::GetRepoTopLevel(command) => Some(&command.repo_id),
        UiCommand::ListBundles(command) => Some(&command.repo_id),
        UiCommand::ClientHello(_) | UiCommand::SetOptions(_) | UiCommand::Unknown => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use im_agent::protocol::{UiCommand, WatchRepoCommand};

    use super::*;

    #[test]
    fn routes_windows_repo_command_to_windows_backend() {
        let mut routes = HashMap::new();
        routes.insert("repo_windows".to_string(), RepoBackend::Windows);

        let command = UiCommand::WatchRepo(WatchRepoCommand {
            repo_id: "repo_windows".to_string(),
        });

        let routed = resolve_repo_backend(&command, &routes).expect("route");
        assert_eq!(routed, Some(RepoBackend::Windows));
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
