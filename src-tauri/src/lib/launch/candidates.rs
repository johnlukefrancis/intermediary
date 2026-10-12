// Path: src-tauri/src/lib/launch/candidates.rs
// Description: Top-level launch candidate discovery for repository roots, ranked by observed usage

use std::collections::HashMap;
use std::path::Path;

use super::recent_items::recent_item_evidence;
use super::unreal_recent::unreal_recent_evidence;
use super::{kind_for_file_name, path_identity, LaunchCandidate, LaunchKind};

/// A `package.json` beside top-level HTML marks it as a build tool's entry page, not a page to open.
const BUILD_MANIFEST: &str = "package.json";

struct RootFiles {
    folder_name: String,
    host_root: String,
    files: Vec<(String, LaunchKind)>,
}

/// Candidates for each host-visible root, in input order; an unreadable root has none.
pub fn discover_candidates(host_roots: &[String]) -> Vec<Vec<LaunchCandidate>> {
    let scanned: Vec<RootFiles> = host_roots.iter().map(|root| scan_root(root)).collect();
    let stems: Vec<String> = scanned
        .iter()
        .flat_map(|root| root.files.iter().map(|(name, _)| file_stem(name)))
        .collect();
    if stems.is_empty() {
        return scanned.iter().map(|_| Vec::new()).collect();
    }
    let mut evidence = recent_item_evidence(&stems);
    for (path, opened_ms) in unreal_recent_evidence() {
        let slot = evidence.entry(path).or_insert(opened_ms);
        *slot = (*slot).max(opened_ms);
    }
    scanned.iter().map(|root| rank(root, &evidence)).collect()
}

fn scan_root(host_root: &str) -> RootFiles {
    let folder_name = Path::new(&host_root.replace('\\', "/"))
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut names = Vec::new();
    let mut has_manifest = false;
    if let Ok(entries) = std::fs::read_dir(host_root) {
        for entry in entries.flatten() {
            if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            has_manifest |= name.eq_ignore_ascii_case(BUILD_MANIFEST);
            names.push(name);
        }
    }
    let files = names
        .into_iter()
        .filter_map(|name| kind_for_file_name(&name).map(|kind| (name, kind)))
        .filter(|(_, kind)| !(has_manifest && *kind == LaunchKind::Html))
        .collect();
    RootFiles {
        folder_name,
        host_root: host_root.to_string(),
        files,
    }
}

fn rank(root: &RootFiles, evidence: &HashMap<String, i64>) -> Vec<LaunchCandidate> {
    let mut candidates: Vec<LaunchCandidate> = root
        .files
        .iter()
        .map(|(name, kind)| LaunchCandidate {
            relative_path: name.clone(),
            kind: *kind,
            last_used_ms: evidence
                .get(&path_identity(&format!("{}\\{}", root.host_root, name)))
                .copied(),
        })
        .collect();
    let folder = root.folder_name.to_lowercase();
    candidates.sort_by(|a, b| {
        b.last_used_ms
            .cmp(&a.last_used_ms)
            .then_with(|| kind_rank(a.kind).cmp(&kind_rank(b.kind)))
            .then_with(|| {
                let a_named = file_stem(&a.relative_path).to_lowercase() == folder;
                let b_named = file_stem(&b.relative_path).to_lowercase() == folder;
                b_named.cmp(&a_named)
            })
            .then_with(|| a.relative_path.to_lowercase().cmp(&b.relative_path.to_lowercase()))
    });
    candidates
}

fn kind_rank(kind: LaunchKind) -> u8 {
    match kind {
        LaunchKind::Unreal => 0,
        LaunchKind::Html => 1,
    }
}

fn file_stem(name: &str) -> String {
    Path::new(name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{rank, RootFiles};
    use crate::launch::LaunchKind;
    use std::collections::HashMap;

    fn root(files: &[(&str, LaunchKind)]) -> RootFiles {
        RootFiles {
            folder_name: "Coilbound".to_string(),
            host_root: r"\\wsl.localhost\Ubuntu\home\johnf\dev\Coilbound".to_string(),
            files: files.iter().map(|(n, k)| (n.to_string(), *k)).collect(),
        }
    }

    #[test]
    fn usage_evidence_outranks_name_rules() {
        let files = root(&[("Coilbound.html", LaunchKind::Html), ("film.html", LaunchKind::Html)]);
        let evidence = HashMap::from([(
            r"\\wsl.localhost\ubuntu\home\johnf\dev\coilbound\film.html".to_string(),
            5,
        )]);
        let ranked = rank(&files, &evidence);
        assert_eq!(ranked[0].relative_path, "film.html");
        assert_eq!(ranked[0].last_used_ms, Some(5));
    }

    #[test]
    fn without_evidence_unreal_then_folder_named_file_wins() {
        let files = root(&[
            ("a.html", LaunchKind::Html),
            ("Coilbound.html", LaunchKind::Html),
            ("Game.uproject", LaunchKind::Unreal),
        ]);
        let ranked: Vec<String> = rank(&files, &HashMap::new())
            .into_iter()
            .map(|c| c.relative_path)
            .collect();
        assert_eq!(ranked, ["Game.uproject", "Coilbound.html", "a.html"]);
    }
}
