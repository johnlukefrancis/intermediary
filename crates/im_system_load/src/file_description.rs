// Path: crates/im_system_load/src/file_description.rs
// Description: Friendly process labels from the executable's FileDescription, cached per image name

use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr;

use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};

use crate::wide::{decode_into, to_wide, wide_len};

const MAX_PATH_UNITS: usize = 32 * 1024;

/// Lookups run once per image name; failures cache the plain name.
#[derive(Default)]
pub struct LabelCache {
    labels: HashMap<String, String>,
    path: Vec<u16>,
    info: Vec<u8>,
}

impl LabelCache {
    pub fn label(&mut self, name: &str, pid: u32) -> String {
        if let Some(label) = self.labels.get(name) {
            return label.clone();
        }
        let label = self.describe(pid).unwrap_or_else(|| name.to_owned());
        self.labels.insert(name.to_owned(), label.clone());
        label
    }

    fn describe(&mut self, pid: u32) -> Option<String> {
        self.image_path(pid)?;
        // SAFETY: `path` is nul-terminated by `image_path`.
        let size = unsafe { GetFileVersionInfoSizeW(self.path.as_ptr(), ptr::null_mut()) };
        if size == 0 {
            return None;
        }
        self.info.clear();
        self.info.resize(size as usize, 0);
        // SAFETY: `info` is writable for `size` bytes.
        let loaded = unsafe {
            GetFileVersionInfoW(self.path.as_ptr(), 0, size, self.info.as_mut_ptr().cast())
        };
        if loaded == 0 {
            return None;
        }
        let (block, len) = self.query(r"\VarFileInfo\Translation")?;
        if len < 4 {
            return None;
        }
        // SAFETY: the translation entry holds at least two u16 values.
        let (language, codepage) = unsafe {
            let units = block.cast::<u16>();
            (units.read_unaligned(), units.add(1).read_unaligned())
        };
        let subblock = format!(r"\StringFileInfo\{language:04x}{codepage:04x}\FileDescription");
        let (text, units) = self.query(&subblock)?;
        // SAFETY: string values are `units` UTF-16 units inside `info`.
        let units = unsafe {
            let text = text.cast::<u16>();
            let len = wide_len(text, units as usize);
            (0..len)
                .map(|index| text.add(index).read_unaligned())
                .collect::<Vec<u16>>()
        };
        let mut label = String::new();
        decode_into(&mut label, &units);
        let label = label.trim();
        (!label.is_empty()).then(|| label.to_owned())
    }

    fn image_path(&mut self, pid: u32) -> Option<()> {
        // SAFETY: limited-information access only; the handle is closed below.
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if process.is_null() {
            return None;
        }
        self.path.clear();
        self.path.resize(MAX_PATH_UNITS, 0);
        let mut len = MAX_PATH_UNITS as u32;
        // SAFETY: `path` is writable for `len` units.
        let named = unsafe {
            QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                self.path.as_mut_ptr(),
                &mut len,
            )
        };
        // SAFETY: `process` came from OpenProcess.
        unsafe { CloseHandle(process) };
        if named == 0 || len as usize >= MAX_PATH_UNITS {
            return None;
        }
        self.path.truncate(len as usize);
        self.path.push(0);
        Some(())
    }

    fn query(&self, subblock: &str) -> Option<(*const c_void, u32)> {
        let wide = to_wide(subblock);
        let mut value: *mut c_void = ptr::null_mut();
        let mut len = 0u32;
        // SAFETY: `info` holds a loaded version block; `value` points into it.
        let found = unsafe {
            VerQueryValueW(
                self.info.as_ptr().cast(),
                wide.as_ptr(),
                &mut value,
                &mut len,
            )
        };
        (found != 0 && !value.is_null()).then_some((value.cast_const(), len))
    }
}
