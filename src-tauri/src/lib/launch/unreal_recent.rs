// Path: src-tauri/src/lib/launch/unreal_recent.rs
// Description: Unreal Editor's own recent-projects evidence from every engine version's EditorSettings.ini

use chrono::NaiveDateTime;
use std::collections::HashMap;
use std::path::PathBuf;

use super::path_identity;

const RECENT_KEY: &str = "RecentlyOpenedProjectFiles=";

fn settings_files() -> Vec<PathBuf> {
    let Some(local) = dirs::data_local_dir().filter(|_| cfg!(windows)) else {
        return Vec::new();
    };
    let Ok(versions) = std::fs::read_dir(local.join("UnrealEngine")) else {
        return Vec::new();
    };
    versions
        .flatten()
        .map(|version| version.path().join(r"Saved\Config\WindowsEditor\EditorSettings.ini"))
        .filter(|path| path.is_file())
        .collect()
}

/// Last-open times keyed by `path_identity` of each `.uproject` the editor remembers.
pub fn unreal_recent_evidence() -> HashMap<String, i64> {
    let mut evidence = HashMap::new();
    for file in settings_files() {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        for (project, opened_ms) in parse_recent_projects(&text) {
            let slot = evidence.entry(path_identity(&project)).or_insert(opened_ms);
            *slot = (*slot).max(opened_ms);
        }
    }
    evidence
}

/// LastOpenTime is the editor's UTC `FDateTime` text, `YYYY.MM.DD-hh.mm.ss`.
pub(super) fn parse_recent_projects(ini: &str) -> Vec<(String, i64)> {
    ini.lines()
        .filter_map(|line| line.trim().strip_prefix(RECENT_KEY))
        .filter_map(|value| {
            let project = quoted_field(value, "ProjectName=")?;
            let time_text = field(value, "LastOpenTime=")?;
            let time = NaiveDateTime::parse_from_str(time_text, "%Y.%m.%d-%H.%M.%S").ok()?;
            Some((project.to_string(), time.and_utc().timestamp_millis()))
        })
        .collect()
}

fn quoted_field<'a>(value: &'a str, key: &str) -> Option<&'a str> {
    let rest = &value[value.find(key)? + key.len()..];
    let rest = rest.strip_prefix('"')?;
    Some(&rest[..rest.find('"')?])
}

fn field<'a>(value: &'a str, key: &str) -> Option<&'a str> {
    let rest = &value[value.find(key)? + key.len()..];
    let end = rest.find([',', ')']).unwrap_or(rest.len());
    Some(&rest[..end])
}

#[cfg(test)]
mod tests {
    use super::parse_recent_projects;

    #[test]
    fn parses_editor_recent_projects() {
        let ini = "[/Script/UnrealEd.EditorSettings]\n\
RecentlyOpenedProjectFiles=(ProjectName=\"E:/Worktrees/blorb-code/Blorb.uproject\",LastOpenTime=2026.10.12-01.03.24)\n\
RecentlyOpenedProjectFiles=(ProjectName=\"broken\")\n";
        let parsed = parse_recent_projects(ini);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0, "E:/Worktrees/blorb-code/Blorb.uproject");
        assert_eq!(parsed[0].1, 1_791_767_004_000);
    }
}
