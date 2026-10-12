// Path: src-tauri/src/lib/launch/unreal_focus.rs
// Description: Finds an UnrealEditor.exe whose command line names a project and brings its window forward

/// What a focus attempt found for one project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorPresence {
    NotRunning,
    Focused,
    /// Running without a top-level window yet (still loading); a second editor must not start.
    Starting,
}

#[cfg(windows)]
pub fn focus_running_editor(project_identity: &str) -> EditorPresence {
    let pids = editor_pids_for(project_identity);
    if pids.is_empty() {
        return EditorPresence::NotRunning;
    }
    match main_window_of(&pids) {
        Some(window) => {
            bring_forward(window);
            EditorPresence::Focused
        }
        None => EditorPresence::Starting,
    }
}

#[cfg(not(windows))]
pub fn focus_running_editor(_project_identity: &str) -> EditorPresence {
    EditorPresence::NotRunning
}

#[cfg(windows)]
fn editor_pids_for(project_identity: &str) -> Vec<u32> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    let mut pids = Vec::new();
    // SAFETY: a process snapshot takes no pointers; the handle is closed below.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return pids;
    }
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    // SAFETY: `entry` is a live PROCESSENTRY32W with dwSize set, as both calls require.
    let mut more = unsafe { Process32FirstW(snapshot, &mut entry) } != 0;
    while more {
        let name_len = entry.szExeFile.iter().position(|u| *u == 0).unwrap_or(0);
        let name = String::from_utf16_lossy(&entry.szExeFile[..name_len]);
        if name.eq_ignore_ascii_case("UnrealEditor.exe")
            && command_line(entry.th32ProcessID)
                .is_some_and(|line| super::path_identity(&line).contains(project_identity))
        {
            pids.push(entry.th32ProcessID);
        }
        // SAFETY: as above.
        more = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
    }
    // SAFETY: `snapshot` is a valid handle owned here and closed once.
    unsafe { CloseHandle(snapshot) };
    pids
}

#[cfg(windows)]
fn command_line(pid: u32) -> Option<String> {
    use windows_sys::Wdk::System::Threading::{
        NtQueryInformationProcess, ProcessCommandLineInformation,
    };
    use windows_sys::Win32::Foundation::{CloseHandle, UNICODE_STRING};
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    // SAFETY: OpenProcess takes no pointers; a null result is checked.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if process.is_null() {
        return None;
    }
    let mut buffer = vec![0u64; 8 * 1024];
    let mut returned = 0u32;
    // SAFETY: the u64 buffer is aligned for UNICODE_STRING and its byte length is passed;
    // on success the kernel writes the string header followed by its characters into it.
    let status = unsafe {
        NtQueryInformationProcess(
            process,
            ProcessCommandLineInformation,
            buffer.as_mut_ptr().cast(),
            (buffer.len() * 8) as u32,
            &mut returned,
        )
    };
    // SAFETY: `process` is a valid handle owned here and closed once.
    unsafe { CloseHandle(process) };
    if status < 0 {
        return None;
    }
    // SAFETY: success guarantees a UNICODE_STRING at the buffer start whose Buffer points
    // inside this same allocation for Length bytes.
    let text = unsafe {
        let header = &*(buffer.as_ptr() as *const UNICODE_STRING);
        if header.Buffer.is_null() {
            return None;
        }
        let units = std::slice::from_raw_parts(header.Buffer, header.Length as usize / 2);
        String::from_utf16_lossy(units)
    };
    Some(text)
}

#[cfg(windows)]
struct WindowSearch<'a> {
    pids: &'a [u32],
    found: windows_sys::Win32::Foundation::HWND,
}

#[cfg(windows)]
unsafe extern "system" fn visit_window(
    window: windows_sys::Win32::Foundation::HWND,
    state: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::core::BOOL {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindow, GetWindowThreadProcessId, IsWindowVisible, GW_OWNER,
    };

    // SAFETY: `state` is the WindowSearch pointer passed by main_window_of for this enumeration.
    let search = unsafe { &mut *(state as *mut WindowSearch) };
    let mut pid = 0u32;
    // SAFETY: `window` comes from EnumWindows and `pid` is a live out-parameter.
    let owned_by_pid = unsafe { GetWindowThreadProcessId(window, &mut pid) } != 0
        && search.pids.contains(&pid);
    // SAFETY: plain queries on a window handle supplied by EnumWindows.
    let is_main = unsafe { IsWindowVisible(window) != 0 && GetWindow(window, GW_OWNER).is_null() };
    if owned_by_pid && is_main {
        search.found = window;
        return 0;
    }
    1
}

#[cfg(windows)]
fn main_window_of(pids: &[u32]) -> Option<windows_sys::Win32::Foundation::HWND> {
    use windows_sys::Win32::UI::WindowsAndMessaging::EnumWindows;

    let mut search = WindowSearch {
        pids,
        found: std::ptr::null_mut(),
    };
    // SAFETY: the callback only dereferences `search`, which outlives this synchronous call.
    unsafe { EnumWindows(Some(visit_window), &mut search as *mut WindowSearch as isize) };
    (!search.found.is_null()).then_some(search.found)
}

#[cfg(windows)]
fn bring_forward(window: windows_sys::Win32::Foundation::HWND) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };

    // SAFETY: plain window-state calls on a live top-level window handle.
    unsafe {
        if IsIconic(window) != 0 {
            ShowWindow(window, SW_RESTORE);
        }
        SetForegroundWindow(window);
    }
}
