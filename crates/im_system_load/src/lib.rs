// Path: crates/im_system_load/src/lib.rs
// Description: Crate root - Windows system-load sampler producing SystemLoadSnapshot

mod error;
pub mod snapshot;

#[cfg(any(windows, test))]
mod gpu_instance;
#[cfg(any(windows, test))]
mod gpu_usage;

#[cfg(windows)]
mod cpu_identity;
#[cfg(windows)]
mod file_description;
#[cfg(windows)]
mod gpu_adapters;
#[cfg(windows)]
mod gpu_counters;
#[cfg(windows)]
mod memory_status;
#[cfg(windows)]
mod pdh_query;
#[cfg(windows)]
mod process_groups;
#[cfg(windows)]
mod process_rates;
#[cfg(windows)]
mod process_table;
#[cfg(windows)]
mod sampler;
#[cfg(windows)]
mod system_counters;
#[cfg(windows)]
mod wide;

#[cfg(not(windows))]
mod unsupported;

pub use error::SystemLoadError;
#[cfg(windows)]
pub use sampler::Sampler;
pub use snapshot::SystemLoadSnapshot;
#[cfg(not(windows))]
pub use unsupported::Sampler;

#[cfg(test)]
mod gpu_instance_tests;
#[cfg(test)]
mod gpu_usage_tests;
