// Path: crates/im_bundle/src/lib.rs
// Description: Library root for bundle scanning and zip creation

pub mod compression_policy;
pub mod error;
pub mod global_excludes;
pub mod manifest;
pub mod plan;
pub mod progress;
pub mod progress_sink;
pub mod scanner;
pub mod writer;

pub use error::{BundleError, Result};
pub use plan::BundlePlan;
pub use writer::{write_bundle, write_bundle_with_progress, BundleResult};

#[cfg(test)]
mod writer_tests;
