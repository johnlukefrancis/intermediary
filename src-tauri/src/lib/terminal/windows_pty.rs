// Path: src-tauri/src/lib/terminal/windows_pty.rs
// Description: Windows ConPTY pipe and master owner feeding the exclusive at-creation process seam

use super::session_spawn::{SpawnError, SpawnedPty};
use super::shell::TerminalCommand;
use super::windows_process;
use im_bundle::process_job::JobHandle;
use portable_pty::{MasterPty, PtySize};
use std::fs::File;
use std::io;
use std::mem;
use std::os::windows::io::{AsRawHandle, FromRawHandle, RawHandle};
use std::ptr;
use std::sync::{Arc, Mutex};
use windows_sys::Win32::Foundation::{SetHandleInformation, HANDLE, HANDLE_FLAG_INHERIT};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::System::Console::{
    ClosePseudoConsole, CreatePseudoConsole, ResizePseudoConsole, COORD, HPCON,
};
use windows_sys::Win32::System::Pipes::CreatePipe;

const CONPTY_FLAGS: u32 = 0x1 | 0x2 | 0x4;

pub fn spawn(
    command: TerminalCommand,
    size: PtySize,
    job: &JobHandle,
) -> Result<SpawnedPty, SpawnError> {
    let (input_read, input_write) = pipe().map_err(openpty_error)?;
    let (output_read, output_write) = pipe().map_err(openpty_error)?;
    let mut console: HPCON = 0;
    // SAFETY: the four pipe handles are live for the call.
    let result = unsafe {
        CreatePseudoConsole(
            coord(size),
            raw(&input_read),
            raw(&output_write),
            CONPTY_FLAGS,
            &mut console,
        )
    };
    if result < 0 {
        return Err(SpawnError::new(
            "openpty",
            format!("Failed to create the Windows pseudoconsole: HRESULT {result:#x}"),
        ));
    }
    drop(input_read);
    drop(output_write);
    let console = Arc::new(ConsoleHandle(console));
    let master = WindowsMaster {
        output: output_read,
        input: Mutex::new(Some(input_write)),
        size: Mutex::new(size),
        console: console.clone(),
    };
    let reader = master.try_clone_reader().map_err(|err| {
        SpawnError::new(
            "clone_reader",
            format!("Failed to clone terminal output: {err}"),
        )
    })?;
    let writer = master.take_writer().map_err(|err| {
        SpawnError::new(
            "take_writer",
            format!("Failed to take terminal input: {err}"),
        )
    })?;
    let child = match windows_process::create(&command, console.0, job) {
        Ok(child) => child,
        Err(err) => {
            drop(writer);
            drop(reader);
            drop(master);
            return Err(SpawnError::new(
                "spawn",
                format!("Failed to start the terminal shell: {err}"),
            ));
        }
    };
    Ok(SpawnedPty {
        master: Box::new(master),
        writer: Box::new(writer),
        reader: Box::new(reader),
        child: Box::new(child),
    })
}

struct ConsoleHandle(HPCON);

unsafe impl Send for ConsoleHandle {}
unsafe impl Sync for ConsoleHandle {}

impl Drop for ConsoleHandle {
    fn drop(&mut self) {
        // SAFETY: this is the single owner of the live HPCON.
        unsafe { ClosePseudoConsole(self.0) };
    }
}

struct WindowsMaster {
    output: File,
    input: Mutex<Option<File>>,
    size: Mutex<PtySize>,
    /// Declared last so every pipe owner drops before ClosePseudoConsole.
    console: Arc<ConsoleHandle>,
}

impl MasterPty for WindowsMaster {
    fn resize(&self, size: PtySize) -> anyhow::Result<()> {
        // SAFETY: the HPCON remains live through `self.console`.
        let result = unsafe { ResizePseudoConsole(self.console.0, coord(size)) };
        if result < 0 {
            anyhow::bail!("ResizePseudoConsole failed: HRESULT {result:#x}");
        }
        *self
            .size
            .lock()
            .map_err(|_| anyhow::anyhow!("PTY size lock poisoned"))? = size;
        Ok(())
    }

    fn get_size(&self) -> anyhow::Result<PtySize> {
        Ok(*self
            .size
            .lock()
            .map_err(|_| anyhow::anyhow!("PTY size lock poisoned"))?)
    }

    fn try_clone_reader(&self) -> anyhow::Result<Box<dyn std::io::Read + Send>> {
        Ok(Box::new(self.output.try_clone()?))
    }

    fn take_writer(&self) -> anyhow::Result<Box<dyn std::io::Write + Send>> {
        self.input
            .lock()
            .map_err(|_| anyhow::anyhow!("PTY input lock poisoned"))?
            .take()
            .map(|writer| Box::new(writer) as Box<dyn std::io::Write + Send>)
            .ok_or_else(|| anyhow::anyhow!("terminal writer was already taken"))
    }
}

fn pipe() -> io::Result<(File, File)> {
    let mut read = ptr::null_mut();
    let mut write = ptr::null_mut();
    let security = SECURITY_ATTRIBUTES {
        nLength: mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: ptr::null_mut(),
        bInheritHandle: 1,
    };
    // SAFETY: output pointers and security descriptor are live for the call.
    if unsafe { CreatePipe(&mut read, &mut write, &security, 0) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: CreatePipe transferred ownership of both handles.
    let read = unsafe { file(read) };
    let write = unsafe { file(write) };
    // SAFETY: inheritance is unnecessary because ConPTY duplicates its ends.
    unsafe {
        if SetHandleInformation(raw(&read), HANDLE_FLAG_INHERIT, 0) == 0
            || SetHandleInformation(raw(&write), HANDLE_FLAG_INHERIT, 0) == 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    Ok((read, write))
}

unsafe fn file(handle: HANDLE) -> File {
    File::from_raw_handle(handle as RawHandle)
}

fn raw(file: &File) -> HANDLE {
    file.as_raw_handle() as HANDLE
}

fn coord(size: PtySize) -> COORD {
    COORD {
        X: size.cols as i16,
        Y: size.rows as i16,
    }
}

fn openpty_error(err: io::Error) -> SpawnError {
    SpawnError::new("openpty", format!("Failed to open terminal pipes: {err}"))
}

#[cfg(test)]
mod tests {
    use super::spawn;
    use crate::terminal::shell::TerminalCommand;
    use im_bundle::process_job::JobHandle;
    use portable_pty::PtySize;
    use std::ffi::OsString;
    use std::time::Duration;

    #[test]
    fn child_belongs_to_the_job_at_create_process_return() {
        let command = TerminalCommand {
            program: r"C:\Windows\System32\ping.exe".into(),
            args: ["-n", "30", "127.0.0.1"].map(OsString::from).to_vec(),
            cwd: std::env::temp_dir(),
            env: std::env::vars_os().collect(),
        };
        let job = JobHandle::create().expect("job");
        let spawned = spawn(
            command,
            PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            },
            &job,
        )
        .expect("spawn");
        assert!(job.active_processes().expect("query membership") >= 1);
        job.terminate_and_observe(Duration::from_secs(2))
            .expect("terminate job");

        let super::SpawnedPty {
            master,
            writer,
            mut reader,
            mut child,
        } = spawned;
        child.wait().expect("wait child");
        drop(writer);
        let closer = std::thread::spawn(move || drop(master));
        let _ = std::io::copy(&mut reader, &mut std::io::sink());
        closer.join().expect("join PTY close");
    }
}
