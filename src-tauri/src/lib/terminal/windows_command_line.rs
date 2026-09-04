// Path: src-tauri/src/lib/terminal/windows_command_line.rs
// Description: Exact UTF-16 command-line and environment encoding for the Windows terminal child

use super::shell::TerminalCommand;
use std::ffi::{OsStr, OsString};
use std::io;
use std::os::windows::ffi::OsStrExt;

pub fn command_line(command: &TerminalCommand) -> io::Result<Vec<u16>> {
    let mut values = Vec::with_capacity(command.args.len() + 1);
    values.push(command.program.as_os_str());
    values.extend(command.args.iter().map(OsString::as_os_str));
    let mut line = Vec::new();
    for (index, value) in values.into_iter().enumerate() {
        if index > 0 {
            line.push(b' ' as u16);
        }
        quote_arg(value, &mut line)?;
    }
    line.push(0);
    Ok(line)
}

pub fn environment_block(env: &[(OsString, OsString)]) -> io::Result<Vec<u16>> {
    let mut entries = env.to_vec();
    entries.sort_by_key(|(key, _)| key.to_string_lossy().to_uppercase());
    let mut block = Vec::new();
    for (key, value) in entries {
        append_without_nul(&key, &mut block)?;
        block.push(b'=' as u16);
        append_without_nul(&value, &mut block)?;
        block.push(0);
    }
    block.push(0);
    Ok(block)
}

pub fn wide_nul(value: &OsStr) -> io::Result<Vec<u16>> {
    let mut wide = Vec::new();
    append_without_nul(value, &mut wide)?;
    wide.push(0);
    Ok(wide)
}

fn quote_arg(value: &OsStr, output: &mut Vec<u16>) -> io::Result<()> {
    let wide = value.encode_wide().collect::<Vec<_>>();
    if wide.contains(&0) {
        return Err(invalid_nul());
    }
    let quoted = wide.is_empty() || wide.iter().any(|ch| matches!(*ch, 9 | 32 | 34));
    if !quoted {
        output.extend(wide);
        return Ok(());
    }
    output.push(34);
    let mut slashes = 0usize;
    for ch in wide {
        if ch == 92 {
            slashes += 1;
        } else if ch == 34 {
            push_slashes(output, slashes * 2 + 1);
            output.push(34);
            slashes = 0;
        } else {
            push_slashes(output, slashes);
            output.push(ch);
            slashes = 0;
        }
    }
    push_slashes(output, slashes * 2);
    output.push(34);
    Ok(())
}

fn append_without_nul(value: &OsStr, output: &mut Vec<u16>) -> io::Result<()> {
    for ch in value.encode_wide() {
        if ch == 0 {
            return Err(invalid_nul());
        }
        output.push(ch);
    }
    Ok(())
}

fn push_slashes(output: &mut Vec<u16>, count: usize) {
    output.extend(std::iter::repeat_n(b'\\' as u16, count));
}

fn invalid_nul() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "Windows process data contains NUL",
    )
}
