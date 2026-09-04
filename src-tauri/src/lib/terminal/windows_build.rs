// Path: src-tauri/src/lib/terminal/windows_build.rs
// Description: Reads the host's CurrentBuildNumber so xterm can enable ConPTY-aware reflow; None wherever it cannot be known

/// `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\CurrentBuildNumber` as a
/// number. The value is informational (it only tunes xterm), so a registry
/// that will not answer yields `None` rather than an error.
#[cfg(windows)]
pub fn current_build_number() -> Option<u32> {
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};

    let subkey = wide(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion");
    let value = wide("CurrentBuildNumber");
    let mut buffer = [0u16; 32];
    let mut size_bytes = (buffer.len() * std::mem::size_of::<u16>()) as u32;
    let mut kind: u32 = 0;
    // SAFETY: every pointer names a live stack buffer that outlives the call;
    // `size_bytes` carries the buffer's byte size in and the bytes written out,
    // and `RRF_RT_REG_SZ` restricts the read to a string that fits it.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            &mut kind,
            buffer.as_mut_ptr().cast(),
            &mut size_bytes,
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let units = (size_bytes as usize / std::mem::size_of::<u16>()).min(buffer.len());
    String::from_utf16_lossy(&buffer[..units])
        .trim_end_matches('\0')
        .trim()
        .parse()
        .ok()
}

#[cfg(windows)]
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// There is no Windows build to report off Windows.
#[cfg(not(windows))]
pub fn current_build_number() -> Option<u32> {
    None
}
