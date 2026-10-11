// Path: crates/im_agent/src/protocol/bundle_inspection.rs
// Description: Selection-aware bundle entry inspection request and response contracts

use im_bundle::inspection::EntryInspection;
use im_bundle::plan::{BundleSelection, GlobalExcludes};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectBundleEntryCommand {
    pub repo_id: String,
    pub path: String,
    pub selection: BundleSelection,
    pub global_excludes: GlobalExcludes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectBundleEntryResult {
    pub repo_id: String,
    pub path: String,
    pub inspection: EntryInspection,
}
