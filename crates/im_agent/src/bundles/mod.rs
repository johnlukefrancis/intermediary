// Path: crates/im_agent/src/bundles/mod.rs
// Description: Bundle helpers for the agent

mod bundle_builder;
mod bundle_builder_blocking;
mod bundle_lister;
mod bundle_progress;
mod git_info;
pub mod ignore_rules;

pub use bundle_builder::{build_bundle, BuildBundleOptions, BuildBundleResult};
pub use bundle_lister::{list_bundles, ListBundlesOptions};

#[cfg(test)]
mod bundle_builder_tests;
