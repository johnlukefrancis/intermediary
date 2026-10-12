// Path: crates/im_system_load/src/pdh_query.rs
// Description: One PDH query with English counter paths, formatted scalar reads and reused wildcard arrays

use std::ptr;

use windows_sys::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
    PdhGetFormattedCounterValue, PdhOpenQueryW, PDH_CSTATUS_INVALID_DATA, PDH_CSTATUS_NEW_DATA,
    PDH_CSTATUS_NO_INSTANCE, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE,
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_INVALID_DATA,
    PDH_MORE_DATA, PDH_NO_DATA,
};

use crate::wide::{decode_into, to_wide, wide_len};
use crate::SystemLoadError;

/// Absent from windows-sys; lets over-100 readings through so callers decide when to clamp.
const PDH_FMT_NOCAP100: u32 = 0x0000_8000;
const FORMAT: u32 = PDH_FMT_DOUBLE | PDH_FMT_NOCAP100;
const MAX_ARRAY_ATTEMPTS: usize = 4;
const MAX_INSTANCE_NAME_UNITS: usize = 1024;

#[derive(Clone, Copy)]
pub struct Counter(PDH_HCOUNTER);

// SAFETY: PDH handles are thread-agnostic; the Sampler mutex serialises all use.
unsafe impl Send for Counter {}

pub struct PdhQuery {
    handle: PDH_HQUERY,
    items: Vec<u64>,
    name: String,
}

// SAFETY: see `Counter`; the query handle is only touched through `&mut self`.
unsafe impl Send for PdhQuery {}

impl PdhQuery {
    pub fn open() -> Result<Self, SystemLoadError> {
        let mut handle: PDH_HQUERY = ptr::null_mut();
        // SAFETY: out-pointer is valid; a null data source selects live counters.
        let status = unsafe { PdhOpenQueryW(ptr::null(), 0, &mut handle) };
        if status != 0 {
            return Err(SystemLoadError::status("PdhOpenQueryW", status));
        }
        Ok(Self {
            handle,
            items: Vec::new(),
            name: String::new(),
        })
    }

    pub fn add(&mut self, path: &str) -> Result<Counter, SystemLoadError> {
        let wide = to_wide(path);
        let mut counter: PDH_HCOUNTER = ptr::null_mut();
        // SAFETY: `wide` is nul-terminated and outlives the call.
        let status = unsafe { PdhAddEnglishCounterW(self.handle, wide.as_ptr(), 0, &mut counter) };
        if status != 0 {
            return Err(SystemLoadError::status(
                format!("PdhAddEnglishCounterW({path})"),
                status,
            ));
        }
        Ok(Counter(counter))
    }

    pub fn collect(&mut self) -> Result<(), SystemLoadError> {
        // SAFETY: the handle stays open for the life of `self`.
        let status = unsafe { PdhCollectQueryData(self.handle) };
        match status {
            0 | PDH_NO_DATA => Ok(()),
            _ => Err(SystemLoadError::status("PdhCollectQueryData", status)),
        }
    }

    /// `None` while a rate counter still lacks its second collection.
    pub fn value(&self, counter: Counter) -> Option<f64> {
        let mut value = PDH_FMT_COUNTERVALUE::default();
        // SAFETY: counter belongs to this query; the type out-pointer is optional.
        let status =
            unsafe { PdhGetFormattedCounterValue(counter.0, FORMAT, ptr::null_mut(), &mut value) };
        // SAFETY: PDH_FMT_DOUBLE fills the double member.
        (status == 0 && valid(value.CStatus)).then_some(unsafe { value.Anonymous.doubleValue })
    }

    /// Calls `visit` for every valid instance of a wildcard counter.
    pub fn for_each(
        &mut self,
        counter: Counter,
        mut visit: impl FnMut(&str, f64),
    ) -> Result<(), SystemLoadError> {
        let Some(count) = self.fill(counter)? else {
            return Ok(());
        };
        let items = self.items.as_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
        for index in 0..count {
            // SAFETY: PDH wrote `count` items at the start of the buffer.
            let item = unsafe { items.add(index).read() };
            if !valid(item.FmtValue.CStatus) || item.szName.is_null() {
                continue;
            }
            // SAFETY: names are nul-terminated strings inside the same buffer.
            let units = unsafe {
                let len = wide_len(item.szName, MAX_INSTANCE_NAME_UNITS);
                std::slice::from_raw_parts(item.szName, len)
            };
            decode_into(&mut self.name, units);
            // SAFETY: PDH_FMT_DOUBLE fills the double member.
            visit(&self.name, unsafe { item.FmtValue.Anonymous.doubleValue });
        }
        Ok(())
    }

    fn fill(&mut self, counter: Counter) -> Result<Option<usize>, SystemLoadError> {
        for _ in 0..MAX_ARRAY_ATTEMPTS {
            let mut bytes = u32::try_from(self.items.len() * 8).unwrap_or(u32::MAX);
            let mut count = 0u32;
            let buffer = if bytes == 0 {
                ptr::null_mut()
            } else {
                self.items
                    .as_mut_ptr()
                    .cast::<PDH_FMT_COUNTERVALUE_ITEM_W>()
            };
            // SAFETY: `buffer` is writable for `bytes` bytes and 8-byte aligned.
            let status = unsafe {
                PdhGetFormattedCounterArrayW(counter.0, FORMAT, &mut bytes, &mut count, buffer)
            };
            match status {
                0 => return Ok(Some(count as usize)),
                PDH_MORE_DATA => self.items.resize((bytes as usize).div_ceil(8), 0),
                PDH_INVALID_DATA
                | PDH_NO_DATA
                | PDH_CSTATUS_INVALID_DATA
                | PDH_CSTATUS_NO_INSTANCE => return Ok(None),
                _ => {
                    return Err(SystemLoadError::status(
                        "PdhGetFormattedCounterArrayW",
                        status,
                    ))
                }
            }
        }
        Err(SystemLoadError::new(
            "PdhGetFormattedCounterArrayW kept outgrowing its buffer",
        ))
    }
}

impl Drop for PdhQuery {
    fn drop(&mut self) {
        // SAFETY: closes the query opened in `open`; its counters close with it.
        unsafe { PdhCloseQuery(self.handle) };
    }
}

fn valid(status: u32) -> bool {
    status == PDH_CSTATUS_VALID_DATA || status == PDH_CSTATUS_NEW_DATA
}
