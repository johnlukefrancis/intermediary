// Path: crates/im_agent/src/bundles/mod.rs
// Description: Bundle helpers for the agent

pub mod ignore_rules;
mod bundle_builder;
mod bundle_builder_blocking;
mod bundle_progress;
mod bundle_lister;
mod git_info;

pub use bundle_builder::{build_bundle, BuildBundleOptions, BuildBundleResult};
pub use bundle_lister::{list_bundles, ListBundlesOptions};

#[cfg(test)]
mod bundle_builder_tests;
