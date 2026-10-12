// Path: crates/im_system_load/src/gpu_adapters.rs
// Description: One-time DXGI adapter enumeration for GPU names, memory totals and LUIDs

use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE, DXGI_ERROR_NOT_FOUND,
};

use crate::gpu_instance::luid_key;
use crate::wide::decode_into;
use crate::SystemLoadError;

pub struct GpuAdapter {
    pub name: String,
    pub luid: u64,
    pub dedicated_total_bytes: u64,
    pub shared_total_bytes: u64,
}

/// Hardware adapters in DXGI order, one per LUID.
pub fn enumerate() -> Result<Vec<GpuAdapter>, SystemLoadError> {
    // SAFETY: plain factory creation; DXGI needs no COM apartment.
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }
        .map_err(|err| SystemLoadError::new(format!("CreateDXGIFactory1 failed: {err}")))?;
    let mut adapters: Vec<GpuAdapter> = Vec::new();
    for index in 0.. {
        // SAFETY: the factory is a live interface owned by this frame.
        let adapter = match unsafe { factory.EnumAdapters1(index) } {
            Ok(adapter) => adapter,
            Err(err) if err.code() == DXGI_ERROR_NOT_FOUND => break,
            Err(err) => {
                return Err(SystemLoadError::new(format!(
                    "IDXGIFactory1::EnumAdapters1({index}) failed: {err}"
                )))
            }
        };
        // SAFETY: the adapter is a live interface returned above.
        let desc = unsafe { adapter.GetDesc1() }.map_err(|err| {
            SystemLoadError::new(format!("IDXGIAdapter1::GetDesc1({index}) failed: {err}"))
        })?;
        if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
            continue;
        }
        let luid = luid_key(desc.AdapterLuid.HighPart as u32, desc.AdapterLuid.LowPart);
        if adapters.iter().any(|known| known.luid == luid) {
            continue;
        }
        let mut name = String::new();
        decode_into(&mut name, &desc.Description);
        adapters.push(GpuAdapter {
            name: name.trim().to_owned(),
            luid,
            dedicated_total_bytes: desc.DedicatedVideoMemory as u64,
            shared_total_bytes: desc.SharedSystemMemory as u64,
        });
    }
    Ok(adapters)
}
