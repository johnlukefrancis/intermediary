// Path: crates/im_system_load/src/memory_status.rs
// Description: Physical memory and commit charge from GlobalMemoryStatusEx

use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

use crate::SystemLoadError;

pub struct MemoryStatus {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub commit_bytes: u64,
    pub commit_limit_bytes: u64,
}

pub fn read() -> Result<MemoryStatus, SystemLoadError> {
    let mut status = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: `status` is a correctly sized MEMORYSTATUSEX.
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
        let code = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        return Err(SystemLoadError::status("GlobalMemoryStatusEx", code as u32));
    }
    Ok(MemoryStatus {
        total_bytes: status.ullTotalPhys,
        available_bytes: status.ullAvailPhys,
        commit_bytes: status
            .ullTotalPageFile
            .saturating_sub(status.ullAvailPageFile),
        commit_limit_bytes: status.ullTotalPageFile,
    })
}
