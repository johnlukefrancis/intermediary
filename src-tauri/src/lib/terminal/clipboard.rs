// Path: src-tauri/src/lib/terminal/clipboard.rs
// Description: Reads CF_UNICODETEXT from the Windows clipboard for terminal paste, because WebView2 cannot read the clipboard without a permission prompt

/// The clipboard's UTF-16 text format; declared here so the build needs no
/// extra `windows-sys` feature for one constant.
#[cfg(windows)]
const CF_UNICODETEXT: u32 = 13;

/// The clipboard's current text, or an empty string when it holds no text.
/// The clipboard is closed on every path once it was opened.
#[cfg(windows)]
pub fn read_text() -> Result<String, String> {
    use std::io;
    use std::ptr;
    use windows_sys::Win32::System::DataExchange::{CloseClipboard, OpenClipboard};

    // SAFETY: a null owner window is documented as allowed; the clipboard is
    // closed below on every path after a successful open.
    if unsafe { OpenClipboard(ptr::null_mut()) } == 0 {
        return Err(format!(
            "Failed to open the clipboard: {}",
            io::Error::last_os_error()
        ));
    }
    let text = read_unicode_text();
    // SAFETY: paired with the successful `OpenClipboard` above, on this thread.
    unsafe {
        CloseClipboard();
    }
    text
}

/// Runs with the clipboard open on the current thread.
#[cfg(windows)]
fn read_unicode_text() -> Result<String, String> {
    use std::io;
    use windows_sys::Win32::System::DataExchange::GetClipboardData;
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

    // SAFETY: the caller holds the clipboard open; a null handle means the
    // format is absent, which is "no text", not a failure.
    let handle = unsafe { GetClipboardData(CF_UNICODETEXT) };
    if handle.is_null() {
        return Ok(String::new());
    }
    // SAFETY: `handle` is a global memory object the clipboard owns while it is
    // open. `GlobalLock` pins it for the reads below and `GlobalUnlock` releases
    // that pin before the function returns.
    let data = unsafe { GlobalLock(handle) } as *const u16;
    if data.is_null() {
        return Err(format!(
            "Failed to lock the clipboard text: {}",
            io::Error::last_os_error()
        ));
    }
    // SAFETY: `handle` is the locked global object; `GlobalSize` only reads its size.
    let max_units = unsafe { GlobalSize(handle) } / std::mem::size_of::<u16>();
    let mut units: Vec<u16> = Vec::new();
    for index in 0..max_units {
        // SAFETY: `index` stays below the allocation size measured above and the
        // memory is pinned by the lock; CF_UNICODETEXT is NUL-terminated by contract.
        let unit = unsafe { *data.add(index) };
        if unit == 0 {
            break;
        }
        units.push(unit);
    }
    // SAFETY: releases the pin taken by the successful `GlobalLock` above.
    unsafe {
        GlobalUnlock(handle);
    }
    Ok(String::from_utf16_lossy(&units))
}

/// The paste route exists for the Windows product; elsewhere it is refused.
#[cfg(not(windows))]
pub fn read_text() -> Result<String, String> {
    Err("Clipboard text is available on Windows hosts only".to_string())
}
