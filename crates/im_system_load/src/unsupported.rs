// Path: crates/im_system_load/src/unsupported.rs
// Description: Non-Windows Sampler whose construction reports that sampling is unavailable

use crate::{SystemLoadError, SystemLoadSnapshot};

pub struct Sampler {
    _private: (),
}

impl Sampler {
    pub fn new() -> Result<Self, SystemLoadError> {
        Err(SystemLoadError::new(
            "System load sampling is only available on Windows",
        ))
    }

    pub fn sample(&mut self) -> Result<SystemLoadSnapshot, SystemLoadError> {
        Err(SystemLoadError::new(
            "System load sampling is only available on Windows",
        ))
    }
}
