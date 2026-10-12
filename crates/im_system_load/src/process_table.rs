// Path: crates/im_system_load/src/process_table.rs
// Description: One NtQuerySystemInformation(SystemProcessInformation) walk per sample into reused records

use windows_sys::Wdk::System::SystemInformation::{
    NtQuerySystemInformation, SystemProcessInformation,
};
use windows_sys::Win32::Foundation::STATUS_INFO_LENGTH_MISMATCH;

use crate::wide::decode_into;
use crate::SystemLoadError;

#[repr(C)]
struct UnicodeString {
    length: u16,
    maximum_length: u16,
    buffer: *const u16,
}

/// phnt `SYSTEM_PROCESS_INFORMATION` without the trailing thread array.
#[repr(C)]
#[allow(dead_code)]
struct ProcessEntry {
    next_entry_offset: u32,
    number_of_threads: u32,
    working_set_private_size: i64,
    hard_fault_count: u32,
    number_of_threads_high_watermark: u32,
    cycle_time: u64,
    create_time: i64,
    user_time: i64,
    kernel_time: i64,
    image_name: UnicodeString,
    base_priority: i32,
    unique_process_id: usize,
    inherited_from_unique_process_id: usize,
    handle_count: u32,
    session_id: u32,
    unique_process_key: usize,
    peak_virtual_size: usize,
    virtual_size: usize,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
    private_page_count: usize,
    read_operation_count: i64,
    write_operation_count: i64,
    other_operation_count: i64,
    read_transfer_count: i64,
    write_transfer_count: i64,
    other_transfer_count: i64,
}

#[cfg(target_pointer_width = "64")]
const _: () = {
    assert!(std::mem::offset_of!(ProcessEntry, create_time) == 0x20);
    assert!(std::mem::offset_of!(ProcessEntry, image_name) == 0x38);
    assert!(std::mem::offset_of!(ProcessEntry, unique_process_id) == 0x50);
    assert!(std::mem::offset_of!(ProcessEntry, read_transfer_count) == 0xE8);
    assert!(std::mem::size_of::<ProcessEntry>() == 0x100);
};

#[derive(Default)]
pub struct ProcessRecord {
    pub pid: u32,
    pub create_time: i64,
    /// User plus kernel time in 100 ns units.
    pub cpu_time: u64,
    pub private_bytes: u64,
    pub io_bytes: u64,
    pub threads: u32,
    /// Image name without a trailing `.exe`.
    pub name: String,
}

const INITIAL_BUFFER_BYTES: usize = 512 * 1024;
const GROWTH_SLACK_BYTES: usize = 64 * 1024;
const MAX_ATTEMPTS: usize = 8;

pub struct ProcessTable {
    buffer: Vec<u64>,
    records: Vec<ProcessRecord>,
    len: usize,
}

impl ProcessTable {
    pub fn new() -> Self {
        Self {
            buffer: vec![0; INITIAL_BUFFER_BYTES / 8],
            records: Vec::new(),
            len: 0,
        }
    }

    pub fn refresh(&mut self) -> Result<&[ProcessRecord], SystemLoadError> {
        let used = self.query()?;
        self.len = 0;
        let base = self.buffer.as_ptr().cast::<u8>();
        let mut offset = 0usize;
        while offset + size_of::<ProcessEntry>() <= used {
            // SAFETY: the entry lies inside the kernel-filled prefix of the buffer.
            let entry = unsafe { base.add(offset).cast::<ProcessEntry>().read_unaligned() };
            self.push(&entry);
            if entry.next_entry_offset == 0 {
                break;
            }
            offset += entry.next_entry_offset as usize;
        }
        Ok(&self.records[..self.len])
    }

    fn query(&mut self) -> Result<usize, SystemLoadError> {
        for _ in 0..MAX_ATTEMPTS {
            let capacity = u32::try_from(self.buffer.len() * 8).unwrap_or(u32::MAX);
            let mut needed = 0u32;
            // SAFETY: the buffer is writable for `capacity` bytes.
            let status = unsafe {
                NtQuerySystemInformation(
                    SystemProcessInformation,
                    self.buffer.as_mut_ptr().cast(),
                    capacity,
                    &mut needed,
                )
            };
            if status == STATUS_INFO_LENGTH_MISMATCH {
                let target = (needed as usize).max(capacity as usize) + GROWTH_SLACK_BYTES;
                self.buffer.resize(target.div_ceil(8), 0);
                continue;
            }
            if status < 0 {
                return Err(SystemLoadError::status(
                    "NtQuerySystemInformation(SystemProcessInformation)",
                    status as u32,
                ));
            }
            return Ok((needed as usize).min(capacity as usize));
        }
        Err(SystemLoadError::new(
            "NtQuerySystemInformation(SystemProcessInformation) kept outgrowing its buffer",
        ))
    }

    fn push(&mut self, entry: &ProcessEntry) {
        if self.len == self.records.len() {
            self.records.push(ProcessRecord::default());
        }
        let record = &mut self.records[self.len];
        self.len += 1;
        record.pid = entry.unique_process_id as u32;
        record.create_time = entry.create_time;
        record.cpu_time = (entry.user_time.max(0) + entry.kernel_time.max(0)) as u64;
        record.private_bytes = entry.working_set_private_size.max(0) as u64;
        record.io_bytes = (entry.read_transfer_count.max(0) as u64)
            .saturating_add(entry.write_transfer_count.max(0) as u64);
        record.threads = entry.number_of_threads;
        let name = &entry.image_name;
        if name.buffer.is_null() || name.length == 0 {
            record.name.clear();
        } else {
            // SAFETY: ImageName points into the same kernel-filled buffer.
            let units =
                unsafe { std::slice::from_raw_parts(name.buffer, usize::from(name.length) / 2) };
            decode_into(&mut record.name, units);
            strip_exe(&mut record.name);
        }
    }
}

fn strip_exe(name: &mut String) {
    let len = name.len();
    if len > 4 && name.is_char_boundary(len - 4) && name[len - 4..].eq_ignore_ascii_case(".exe") {
        name.truncate(len - 4);
    }
}
