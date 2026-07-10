// Path: crates/im_bundle/src/lib.rs
// Description: Library root for bundle scanning and zip creation

pub mod cancel;
pub mod compression_policy;
pub mod error;
pub mod git_capture;
pub mod global_excludes;
pub mod global_excludes_summary;
pub mod manifest;
pub mod plan;
pub mod progress;
pub mod progress_sink;
pub mod scanner;
pub(crate) mod selection;
pub mod writer;
pub(crate) mod zip_entry;

pub use error::{BundleError, Result};
pub use plan::BundlePlan;
pub use writer::{
    write_bundle, write_bundle_with_progress, write_bundle_with_progress_and_cancel, BundleResult,
};

#[cfg(test)]
mod writer_tests;
