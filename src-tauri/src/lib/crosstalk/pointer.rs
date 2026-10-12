// Path: src-tauri/src/lib/crosstalk/pointer.rs
// Description: Visible-companion mouse hook yields only to a clicked overlapping window

use std::{cell::Cell, ptr::null_mut};
use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CallNextHookEx, GetAncestor, GetWindow, GetWindowLongPtrW, GetWindowRect, IsWindowVisible,
        SetWindowPos, SetWindowsHookExW, UnhookWindowsHookEx, WindowFromPoint, GA_ROOT,
        GWL_EXSTYLE, GW_OWNER, HHOOK, MSLLHOOKSTRUCT, SWP_NOACTIVATE, SWP_NOMOVE,
        SWP_NOOWNERZORDER, SWP_NOSIZE, WH_MOUSE_LL, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_RBUTTONDOWN,
        WM_XBUTTONDOWN, WS_EX_TOPMOST,
    },
};

#[derive(Clone, Copy)]
struct Hook {
    surface: HWND,
    handle: HHOOK,
}

thread_local! {
    static POINTER: Cell<Option<Hook>> = const { Cell::new(None) };
}

pub fn track(surface: HWND, visible: bool) -> Result<(), String> {
    POINTER.with(|slot| match (slot.get(), visible) {
        (None, true) => {
            let module = unsafe { GetModuleHandleW(std::ptr::null()) };
            if module.is_null() {
                return Err(format!(
                    "Could not resolve Crosstalk module: {}",
                    std::io::Error::last_os_error()
                ));
            }
            let handle = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse), module, 0) };
            if handle.is_null() {
                return Err(format!(
                    "Could not observe Crosstalk clicks: {}",
                    std::io::Error::last_os_error()
                ));
            }
            slot.set(Some(Hook { surface, handle }));
            Ok(())
        }
        (Some(hook), false) => {
            if unsafe { UnhookWindowsHookEx(hook.handle) } == 0 {
                return Err(format!(
                    "Could not release Crosstalk clicks: {}",
                    std::io::Error::last_os_error()
                ));
            }
            slot.set(None);
            Ok(())
        }
        _ => Ok(()),
    })
}

pub fn stop() {
    if let Err(error) = track(null_mut(), false) {
        super::report(&error);
    }
}

unsafe extern "system" fn mouse(code: i32, kind: usize, data: isize) -> isize {
    if code == 0
        && matches!(
            kind as u32,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
        )
    {
        POINTER.with(|slot| {
            if let Some(hook) = slot.get() {
                yield_to_click(hook.surface, &*(data as *const MSLLHOOKSTRUCT));
            }
        });
    }
    CallNextHookEx(null_mut(), code, kind, data)
}

unsafe fn yield_to_click(surface: HWND, event: &MSLLHOOKSTRUCT) {
    if IsWindowVisible(surface) == 0 {
        return;
    }
    let target = GetAncestor(WindowFromPoint(event.pt), GA_ROOT);
    if target.is_null()
        || target == surface
        || target == GetWindow(surface, GW_OWNER)
        || GetWindowLongPtrW(target, GWL_EXSTYLE) & WS_EX_TOPMOST as isize != 0
    {
        return;
    }
    let mut bubble: RECT = std::mem::zeroed();
    let mut clicked: RECT = std::mem::zeroed();
    if GetWindowRect(surface, &mut bubble) == 0 || GetWindowRect(target, &mut clicked) == 0 {
        return;
    }
    if bubble.left < clicked.right
        && bubble.right > clicked.left
        && bubble.top < clicked.bottom
        && bubble.bottom > clicked.top
    {
        // Lower only this companion; the click itself retains normal input handling.
        if SetWindowPos(
            surface,
            target,
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_NOOWNERZORDER,
        ) == 0
        {
            super::report(&format!(
                "Could not yield Crosstalk to clicked window: {}",
                std::io::Error::last_os_error()
            ));
        }
    }
}
