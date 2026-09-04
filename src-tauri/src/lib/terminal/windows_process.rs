// Path: src-tauri/src/lib/terminal/windows_process.rs
// Description: CreateProcessW owner applying ConPTY and Job-list attributes in one exclusive creation call

use super::shell::TerminalCommand;
use super::windows_command_line::{command_line, environment_block, wide_nul};
use im_bundle::process_job::JobHandle;
use portable_pty::{Child, ChildKiller, ExitStatus};
use std::ffi::c_void;
use std::io;
use std::mem;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle};
use std::ptr;
use std::sync::Arc;
use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE, STILL_ACTIVE, WAIT_FAILED};
use windows_sys::Win32::System::Console::HPCON;
use windows_sys::Win32::System::Threading::{
    CreateProcessW, DeleteProcThreadAttributeList, GetExitCodeProcess,
    InitializeProcThreadAttributeList, TerminateProcess, UpdateProcThreadAttribute,
    WaitForSingleObject, CREATE_UNICODE_ENVIRONMENT, EXTENDED_STARTUPINFO_PRESENT, INFINITE,
    PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_JOB_LIST, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
    STARTF_USESTDHANDLES, STARTUPINFOEXW,
};

#[derive(Debug)]
pub struct WindowsChild {
    process: Arc<OwnedHandle>,
    pid: u32,
}

#[derive(Debug)]
struct WindowsKiller(Arc<OwnedHandle>);

pub fn create(
    command: &TerminalCommand,
    console: HPCON,
    job: &JobHandle,
) -> io::Result<WindowsChild> {
    let mut attributes = AttributeList::new(2)?;
    // The pseudoconsole attribute's value is the HPCON itself. Job-list is
    // different: its value is a pointer to an array of Job handles.
    attributes.set(
        PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
        console as *const c_void,
        mem::size_of::<HPCON>(),
    )?;
    let jobs = [job.raw_handle() as HANDLE];
    attributes.set(
        PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
        jobs.as_ptr().cast(),
        mem::size_of::<HANDLE>(),
    )?;
    let mut startup = STARTUPINFOEXW::default();
    startup.StartupInfo.cb = mem::size_of::<STARTUPINFOEXW>() as u32;
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = INVALID_HANDLE_VALUE;
    startup.StartupInfo.hStdOutput = INVALID_HANDLE_VALUE;
    startup.StartupInfo.hStdError = INVALID_HANDLE_VALUE;
    startup.lpAttributeList = attributes.ptr();
    let mut process = PROCESS_INFORMATION::default();
    let application = wide_nul(command.program.as_os_str())?;
    let mut command_line = command_line(command)?;
    let environment = environment_block(&command.env)?;
    let cwd = wide_nul(command.cwd.as_os_str())?;
    // SAFETY: all buffers and both attribute values remain live for the call.
    let created = unsafe {
        CreateProcessW(
            application.as_ptr(),
            command_line.as_mut_ptr(),
            ptr::null(),
            ptr::null(),
            0,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT,
            environment.as_ptr().cast(),
            cwd.as_ptr(),
            &startup.StartupInfo,
            &mut process,
        )
    };
    if created == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: CreateProcessW returned ownership of both handles.
    let process_handle = unsafe { OwnedHandle::from_raw_handle(process.hProcess as RawHandle) };
    let thread_handle = unsafe { OwnedHandle::from_raw_handle(process.hThread as RawHandle) };
    drop(thread_handle);
    Ok(WindowsChild {
        process: Arc::new(process_handle),
        pid: process.dwProcessId,
    })
}

impl ChildKiller for WindowsChild {
    fn kill(&mut self) -> io::Result<()> {
        terminate(&self.process)
    }

    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
        Box::new(WindowsKiller(self.process.clone()))
    }
}

impl ChildKiller for WindowsKiller {
    fn kill(&mut self) -> io::Result<()> {
        terminate(&self.0)
    }

    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
        Box::new(WindowsKiller(self.0.clone()))
    }
}

impl Child for WindowsChild {
    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        exit_status(&self.process)
    }

    fn wait(&mut self) -> io::Result<ExitStatus> {
        // SAFETY: the process handle is live and waitable.
        if unsafe { WaitForSingleObject(raw(&self.process), INFINITE) } == WAIT_FAILED {
            return Err(io::Error::last_os_error());
        }
        exit_status(&self.process)?.ok_or_else(|| io::Error::other("process wait returned active"))
    }

    fn process_id(&self) -> Option<u32> {
        Some(self.pid)
    }

    fn as_raw_handle(&self) -> Option<RawHandle> {
        Some(self.process.as_raw_handle())
    }
}

fn terminate(process: &OwnedHandle) -> io::Result<()> {
    // SAFETY: the owned process handle is live.
    if unsafe { TerminateProcess(raw(process), 1) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn exit_status(process: &OwnedHandle) -> io::Result<Option<ExitStatus>> {
    let mut code = 0u32;
    // SAFETY: the process handle and output pointer are live.
    if unsafe { GetExitCodeProcess(raw(process), &mut code) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((code != STILL_ACTIVE as u32).then(|| ExitStatus::with_exit_code(code)))
}

fn raw(handle: &OwnedHandle) -> HANDLE {
    handle.as_raw_handle() as HANDLE
}

struct AttributeList(Vec<usize>);

impl AttributeList {
    fn new(count: u32) -> io::Result<Self> {
        let mut bytes = 0usize;
        // SAFETY: first call obtains the required byte count.
        unsafe { InitializeProcThreadAttributeList(ptr::null_mut(), count, 0, &mut bytes) };
        let words = bytes.div_ceil(mem::size_of::<usize>());
        let mut data = vec![0usize; words];
        // SAFETY: data is writable for the reported byte count.
        if unsafe {
            InitializeProcThreadAttributeList(data.as_mut_ptr().cast(), count, 0, &mut bytes)
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(data))
    }

    fn ptr(&mut self) -> *mut c_void {
        self.0.as_mut_ptr().cast()
    }

    fn set(&mut self, attribute: usize, value: *const c_void, size: usize) -> io::Result<()> {
        // SAFETY: value points to a live value of `size` bytes for this call.
        if unsafe {
            UpdateProcThreadAttribute(
                self.ptr(),
                0,
                attribute,
                value,
                size,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl Drop for AttributeList {
    fn drop(&mut self) {
        // SAFETY: initialized once and deleted once.
        unsafe { DeleteProcThreadAttributeList(self.ptr()) };
    }
}
