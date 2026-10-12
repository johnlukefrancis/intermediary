// Path: crates/im_system_load/src/cpu_identity.rs
// Description: Processor brand string from the registry and the logical processor count

use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
use windows_sys::Win32::System::Threading::{GetActiveProcessorCount, ALL_PROCESSOR_GROUPS};

use crate::wide::{decode_into, to_wide};
use crate::SystemLoadError;

pub fn processor_name() -> Option<String> {
    let key = to_wide(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0");
    let value = to_wide("ProcessorNameString");
    let mut buffer = [0u16; 256];
    let mut bytes = (buffer.len() * 2) as u32;
    // SAFETY: names are nul-terminated; `buffer` is writable for `bytes` bytes.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    if status != 0 {
        return None;
    }
    let mut name = String::new();
    decode_into(&mut name, &buffer[..(bytes as usize / 2).min(buffer.len())]);
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_owned())
}

/// Logical processors across every processor group.
pub fn logical_processor_count() -> Result<u32, SystemLoadError> {
    // SAFETY: no pointers involved.
    match unsafe { GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) } {
        0 => {
            let code = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            Err(SystemLoadError::status(
                "GetActiveProcessorCount",
                code as u32,
            ))
        }
        count => Ok(count),
    }
}
