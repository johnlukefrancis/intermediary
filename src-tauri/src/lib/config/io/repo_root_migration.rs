// Path: src-tauri/src/lib/config/io/repo_root_migration.rs
// Description: Legacy repository root migration helpers for config loading

use crate::paths::repo_root_resolver::{resolve_legacy_repo_root_from_input, RepoRootKind};
use serde_json::{json, Value};

pub(super) fn migrate_legacy_repo_roots(raw: &mut Value) -> bool {
    let Some(config_obj) = raw.as_object_mut() else {
        return false;
    };
    let Some(repos_value) = config_obj.get_mut("repos") else {
        return false;
    };
    let Some(repos) = repos_value.as_array_mut() else {
        return false;
    };

    let mut changed = false;
    for repo in repos {
        let Some(repo_obj) = repo.as_object_mut() else {
            continue;
        };

        if let Some(root_value) = repo_obj.get_mut("root") {
            changed = migrate_root_value(root_value) || changed;
            if repo_obj.remove("wslPath").is_some() {
                changed = true;
            }
            continue;
        }

        let Some(legacy_path) = repo_obj.get("wslPath").and_then(Value::as_str) else {
            continue;
        };
        let Some(resolved_root) = resolve_legacy_repo_root_from_input(legacy_path) else {
            continue;
        };

        repo_obj.insert(
            "root".to_string(),
            root_json_for_kind(resolved_root.kind, resolved_root.path),
        );
        repo_obj.remove("wslPath");
        changed = true;
    }

    changed
}

fn migrate_root_value(root_value: &mut Value) -> bool {
    let current_kind = root_value
        .as_object()
        .and_then(|root_obj| root_obj.get("kind"))
        .and_then(Value::as_str);
    let current_path = root_value
        .as_object()
        .and_then(|root_obj| root_obj.get("path"))
        .and_then(Value::as_str);

    let replacement = match (current_kind, current_path) {
        (Some("wsl"), Some(path)) => {
            resolve_legacy_repo_root_from_input(path).and_then(|resolved_root| {
                let next_root = root_json_for_kind(resolved_root.kind, resolved_root.path);
                if root_value != &next_root {
                    Some(next_root)
                } else {
                    None
                }
            })
        }
        (Some("windows"), Some(path)) => {
            let migrated_path = resolve_legacy_repo_root_from_input(path)
                .filter(|resolved_root| resolved_root.kind == RepoRootKind::Host)
                .map(|resolved_root| resolved_root.path)
                .unwrap_or_else(|| path.trim().to_string());
            let next_root = json!({ "kind": "host", "path": migrated_path });
            if root_value != &next_root {
                Some(next_root)
            } else {
                None
            }
        }
        (Some("host"), Some(path)) => {
            let trimmed = path.trim();
            let next_root = json!({ "kind": "host", "path": trimmed });
            if root_value != &next_root {
                Some(next_root)
            } else {
                None
            }
        }
        _ => None,
    };

    if let Some(new_root) = replacement {
        *root_value = new_root;
        return true;
    }
    false
}

fn root_json_for_kind(kind: RepoRootKind, path: String) -> Value {
    match kind {
        RepoRootKind::Wsl => json!({ "kind": "wsl", "path": path }),
        RepoRootKind::Host => json!({ "kind": "host", "path": path }),
    }
}
