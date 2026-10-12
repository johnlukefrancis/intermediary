// Path: src-tauri/src/lib/commands/windows_shell.rs
// Description: Native Windows Shell dispatch for item selection and default file or URL opening

use std::ptr::{null, null_mut};
use windows_sys::Win32::System::Com::{
    CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
};
use windows_sys::Win32::UI::Shell::{
    SHOpenFolderAndSelectItems, SHParseDisplayName, ShellExecuteW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

struct Apartment;

impl Apartment {
    fn enter() -> Result<Self, String> {
        let result = unsafe {
            CoInitializeEx(
                null(),
                (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
            )
        };
        check_hresult(result, "Initialize Windows Shell")?;
        Ok(Self)
    }
}

impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

fn wide(value: &str) -> Result<Vec<u16>, String> {
    if value.contains('\0') {
        return Err("Shell target contains a NUL character".to_string());
    }
    Ok(value.encode_utf16().chain(Some(0)).collect())
}

fn check_hresult(result: i32, action: &str) -> Result<(), String> {
    if result < 0 {
        Err(format!("{action} failed (HRESULT 0x{:08X})", result as u32))
    } else {
        Ok(())
    }
}

pub(crate) fn reveal(host_path: &str) -> Result<(), String> {
    let _apartment = Apartment::enter()?;
    let path = wide(host_path)?;
    let mut item = null_mut();
    let parsed = unsafe { SHParseDisplayName(path.as_ptr(), null_mut(), &mut item, 0, null_mut()) };
    check_hresult(parsed, &format!("Resolve '{host_path}' in Windows Shell"))?;
    // With zero children the absolute item identifies the selection in its parent.
    let result = unsafe { SHOpenFolderAndSelectItems(item, 0, null(), 0) };
    unsafe { CoTaskMemFree(item.cast()) };
    check_hresult(result, &format!("Reveal '{host_path}' in Explorer"))
}

pub(crate) fn open(target: &str) -> Result<(), String> {
    let _apartment = Apartment::enter()?;
    let target_wide = wide(target)?;
    let verb = wide("open")?;
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            verb.as_ptr(),
            target_wide.as_ptr(),
            null(),
            null(),
            SW_SHOWNORMAL,
        )
    } as isize;
    if result <= 32 {
        Err(format!(
            "Open '{target}' with its default application failed (Shell code {result})"
        ))
    } else {
        Ok(())
    }
}
