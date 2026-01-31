// Path: crates/im_zip/src/lib.rs
// Description: Library root - zip archive creation API

pub mod error;
pub mod plan;
pub mod progress;
pub mod writer;

// Re-export main types for convenience
pub use error::{Result, ZipError};
pub use plan::ZipPlan;
pub use writer::write_zip;
