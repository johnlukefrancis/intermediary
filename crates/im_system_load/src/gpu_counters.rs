// Path: crates/im_system_load/src/gpu_counters.rs
// Description: PDH GPU engine and memory counters joined to DXGI adapters and process ids

use std::collections::HashMap;

use crate::gpu_adapters::GpuAdapter;
use crate::gpu_instance::{parse_adapter_memory, parse_engine, parse_process_memory};
use crate::gpu_usage::GpuUsage;
use crate::pdh_query::{Counter, PdhQuery};
use crate::snapshot::GpuLoad;
use crate::SystemLoadError;

pub struct GpuCounters {
    engine: Counter,
    adapter_dedicated: Counter,
    adapter_shared: Counter,
    process_local: Counter,
    usage: GpuUsage,
    dedicated_used: HashMap<u64, u64>,
    shared_used: HashMap<u64, u64>,
    process_memory: HashMap<u32, u64>,
}

impl GpuCounters {
    pub fn add(query: &mut PdhQuery) -> Result<Self, SystemLoadError> {
        Ok(Self {
            engine: query.add(r"\GPU Engine(*)\Utilization Percentage")?,
            adapter_dedicated: query.add(r"\GPU Adapter Memory(*)\Dedicated Usage")?,
            adapter_shared: query.add(r"\GPU Adapter Memory(*)\Shared Usage")?,
            // Per-process Dedicated Usage is unbounded for dwm (26 GB on a 16 GB card); Local is resident VRAM.
            process_local: query.add(r"\GPU Process Memory(*)\Local Usage")?,
            usage: GpuUsage::default(),
            dedicated_used: HashMap::new(),
            shared_used: HashMap::new(),
            process_memory: HashMap::new(),
        })
    }

    /// Reads every GPU counter from the query's latest collection.
    pub fn read(&mut self, query: &mut PdhQuery) -> Result<(), SystemLoadError> {
        let usage = &mut self.usage;
        usage.begin();
        query.for_each(self.engine, |name, percent| {
            if let Some(instance) = parse_engine(name) {
                usage.add(&instance, percent);
            }
        })?;
        usage.finish();

        read_adapter_memory(query, self.adapter_dedicated, &mut self.dedicated_used)?;
        read_adapter_memory(query, self.adapter_shared, &mut self.shared_used)?;

        let process_memory = &mut self.process_memory;
        process_memory.clear();
        query.for_each(self.process_local, |name, bytes| {
            if let Some(instance) = parse_process_memory(name) {
                *process_memory.entry(instance.pid).or_insert(0) += to_bytes(bytes);
            }
        })
    }

    pub fn process_percent(&self, pid: u32) -> f32 {
        self.usage.process_percent(pid)
    }

    pub fn process_memory_bytes(&self, pid: u32) -> u64 {
        self.process_memory.get(&pid).copied().unwrap_or(0)
    }

    pub fn gpus(&self, adapters: &[GpuAdapter]) -> Vec<GpuLoad> {
        adapters
            .iter()
            .map(|adapter| {
                let usage = self.usage.adapter(adapter.luid);
                GpuLoad {
                    name: adapter.name.clone(),
                    usage_percent: usage.usage_percent,
                    engines: usage.engines,
                    dedicated_used_bytes: self
                        .dedicated_used
                        .get(&adapter.luid)
                        .copied()
                        .unwrap_or(0),
                    dedicated_total_bytes: adapter.dedicated_total_bytes,
                    shared_used_bytes: self.shared_used.get(&adapter.luid).copied().unwrap_or(0),
                    shared_total_bytes: adapter.shared_total_bytes,
                }
            })
            .collect()
    }
}

/// Sums every physical node of an adapter into its LUID.
fn read_adapter_memory(
    query: &mut PdhQuery,
    counter: Counter,
    used: &mut HashMap<u64, u64>,
) -> Result<(), SystemLoadError> {
    used.clear();
    query.for_each(counter, |name, bytes| {
        if let Some(instance) = parse_adapter_memory(name) {
            *used.entry(instance.luid).or_insert(0) += to_bytes(bytes);
        }
    })
}

fn to_bytes(value: f64) -> u64 {
    value.max(0.0) as u64
}
