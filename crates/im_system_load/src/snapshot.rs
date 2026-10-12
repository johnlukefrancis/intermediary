// Path: crates/im_system_load/src/snapshot.rs
// Description: System-load wire snapshot serialized to the frontend (mirrors app/src/shared/protocol_system_load.ts)

use serde::Serialize;

/// One machine-wide sample. Rates are computed over `interval_ms`, which is 0 (and every rate 0)
/// on a sampler's first sample.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemLoadSnapshot {
    pub sampled_at_ms: u64,
    pub interval_ms: u64,
    pub cpu: CpuLoad,
    pub memory: MemoryLoad,
    pub gpus: Vec<GpuLoad>,
    pub disks: Vec<DiskLoad>,
    pub networks: Vec<NetworkLoad>,
    pub processes: Vec<ProcessGroupLoad>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuLoad {
    pub name: Option<String>,
    pub usage_percent: f32,
    /// One entry per logical processor, in processor order.
    pub core_percents: Vec<f32>,
    pub speed_mhz: Option<f32>,
    pub base_speed_mhz: Option<f32>,
    pub process_count: u32,
    pub thread_count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryLoad {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub commit_bytes: u64,
    pub commit_limit_bytes: u64,
    /// Hard-fault page reads per second (`\Memory\Pages Input/sec`).
    pub hard_faults_per_sec: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuLoad {
    pub name: String,
    /// Busiest engine of this adapter, as Task Manager reports it.
    pub usage_percent: f32,
    /// Busiest engine per engine type ("3D", "Copy", "VideoDecode", ...), busiest first.
    pub engines: Vec<GpuEngineLoad>,
    pub dedicated_used_bytes: u64,
    pub dedicated_total_bytes: u64,
    pub shared_used_bytes: u64,
    pub shared_total_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuEngineLoad {
    pub kind: String,
    pub usage_percent: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskLoad {
    /// PDH physical-disk instance, e.g. "0 C: D:".
    pub name: String,
    pub active_percent: f32,
    pub read_bytes_per_sec: f64,
    pub write_bytes_per_sec: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkLoad {
    pub name: String,
    pub received_bytes_per_sec: f64,
    pub sent_bytes_per_sec: f64,
    /// Link speed in bits per second; 0 when the adapter does not report one.
    pub link_bits_per_sec: f64,
}

/// All processes sharing one image name, e.g. every `chrome.exe`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessGroupLoad {
    /// Image name without the `.exe` suffix.
    pub name: String,
    /// Executable `FileDescription` when available, otherwise `name`.
    pub label: String,
    pub count: u32,
    /// Share of all logical processors, 0..=100.
    pub cpu_percent: f32,
    /// Sum of private working sets (Task Manager's Memory column).
    pub memory_bytes: u64,
    pub io_bytes_per_sec: f64,
    /// Busiest GPU engine share of any process in the group.
    pub gpu_percent: f32,
    pub gpu_memory_bytes: u64,
}
