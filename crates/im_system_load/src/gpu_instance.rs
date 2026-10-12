// Path: crates/im_system_load/src/gpu_instance.rs
// Description: Parses PDH GPU counter instance names into pid, adapter LUID, node and engine

/// `pid_<pid>_luid_<high>_<low>_phys_<n>_eng_<n>_engtype_<type>` from `\GPU Engine(*)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineInstance<'a> {
    pub pid: u32,
    pub luid: u64,
    pub phys: u32,
    pub eng: u32,
    pub engtype: &'a str,
}

/// `pid_<pid>_luid_<high>_<low>_phys_<n>` from `\GPU Process Memory(*)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessMemoryInstance {
    pub pid: u32,
    pub luid: u64,
}

/// `luid_<high>_<low>_phys_<n>` from `\GPU Adapter Memory(*)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdapterMemoryInstance {
    pub luid: u64,
    pub phys: u32,
}

pub fn parse_engine(name: &str) -> Option<EngineInstance<'_>> {
    let mut parts = name.splitn(11, '_');
    let pid = tagged_decimal(&mut parts, "pid")?;
    let luid = tagged_luid(&mut parts)?;
    let phys = tagged_decimal(&mut parts, "phys")?;
    let eng = tagged_decimal(&mut parts, "eng")?;
    if parts.next()? != "engtype" {
        return None;
    }
    let engtype = parts.next().filter(|kind| !kind.is_empty())?;
    Some(EngineInstance {
        pid,
        luid,
        phys,
        eng,
        engtype,
    })
}

pub fn parse_process_memory(name: &str) -> Option<ProcessMemoryInstance> {
    let mut parts = name.split('_');
    let pid = tagged_decimal(&mut parts, "pid")?;
    let luid = tagged_luid(&mut parts)?;
    tagged_decimal(&mut parts, "phys")?;
    parts
        .next()
        .is_none()
        .then_some(ProcessMemoryInstance { pid, luid })
}

pub fn parse_adapter_memory(name: &str) -> Option<AdapterMemoryInstance> {
    let mut parts = name.split('_');
    let luid = tagged_luid(&mut parts)?;
    let phys = tagged_decimal(&mut parts, "phys")?;
    parts
        .next()
        .is_none()
        .then_some(AdapterMemoryInstance { luid, phys })
}

/// Packs a LUID the way DXGI reports it: high part in the upper 32 bits.
pub fn luid_key(high: u32, low: u32) -> u64 {
    (u64::from(high) << 32) | u64::from(low)
}

fn tagged_decimal<'a>(parts: &mut impl Iterator<Item = &'a str>, tag: &str) -> Option<u32> {
    if parts.next()? != tag {
        return None;
    }
    parts.next()?.parse().ok()
}

fn tagged_luid<'a>(parts: &mut impl Iterator<Item = &'a str>) -> Option<u64> {
    if parts.next()? != "luid" {
        return None;
    }
    let high = hex_u32(parts.next()?)?;
    let low = hex_u32(parts.next()?)?;
    Some(luid_key(high, low))
}

fn hex_u32(text: &str) -> Option<u32> {
    let digits = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))?;
    u32::from_str_radix(digits, 16).ok()
}
