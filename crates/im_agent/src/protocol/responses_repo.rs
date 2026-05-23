// Path: crates/im_agent/src/protocol/responses_repo.rs
// Description: Repository topology and directory listing response payloads

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRepoTopLevelResult {
    pub repo_id: String,
    pub dirs: Vec<String>,
    pub files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdirs: Option<std::collections::HashMap<String, Vec<String>>>,
    #[serde(default)]
    pub default_excluded: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRepoDirectoryResult {
    pub repo_id: String,
    pub path: String,
    pub dirs: Vec<String>,
    pub files: Vec<String>,
}
