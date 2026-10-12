// Path: crates/im_system_load/src/error.rs
// Description: Sampler error carrying the failed operation and its OS status

use std::fmt;

#[derive(Debug, Clone)]
pub struct SystemLoadError {
    message: String,
}

impl SystemLoadError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    #[cfg(windows)]
    pub(crate) fn status(operation: impl fmt::Display, status: u32) -> Self {
        Self::new(format!("{operation} failed (status 0x{status:08X})"))
    }
}

impl fmt::Display for SystemLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SystemLoadError {}
