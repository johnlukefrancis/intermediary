// Path: crates/im_bundle/src/lib.rs
// Description: Library root for bundle scanning and zip creation

pub mod error;
pub mod compression_policy;
pub mod global_excludes;
pub mod manifest;
pub mod plan;
pub mod progress;
pub mod scanner;
pub mod writer;

pub use error::{BundleError, Result};
pub use plan::BundlePlan;
pub use writer::{write_bundle, BundleResult};
