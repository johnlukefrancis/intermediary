// Path: crates/im_system_load/src/gpu_usage.rs
// Description: Task Manager GPU aggregation - per-engine sums, adapter busiest engine, process busiest engine

use std::collections::HashMap;

use crate::gpu_instance::EngineInstance;
use crate::snapshot::GpuEngineLoad;

/// Physical engine identity: adapter LUID, node and engine index.
type EngineKey = (u64, u32, u32);

struct EngineTotal {
    engtype: String,
    percent: f64,
    live: bool,
}

/// Engine and process utilisation for one PDH collection; entries persist across samples.
#[derive(Default)]
pub struct GpuUsage {
    engines: HashMap<EngineKey, EngineTotal>,
    processes: HashMap<u32, f64>,
}

pub struct AdapterUsage {
    pub usage_percent: f32,
    pub engines: Vec<GpuEngineLoad>,
}

impl GpuUsage {
    pub fn begin(&mut self) {
        for total in self.engines.values_mut() {
            total.percent = 0.0;
            total.live = false;
        }
        self.processes.clear();
    }

    pub fn add(&mut self, instance: &EngineInstance<'_>, percent: f64) {
        let percent = percent.max(0.0);
        let key = (instance.luid, instance.phys, instance.eng);
        let total = self.engines.entry(key).or_insert_with(|| EngineTotal {
            engtype: instance.engtype.to_owned(),
            percent: 0.0,
            live: false,
        });
        total.percent += percent;
        total.live = true;
        let process = self.processes.entry(instance.pid).or_insert(0.0);
        *process = process.max(percent);
    }

    /// Drops engines that no process reported in this collection.
    pub fn finish(&mut self) {
        self.engines.retain(|_, total| total.live);
    }

    pub fn process_percent(&self, pid: u32) -> f32 {
        self.processes
            .get(&pid)
            .map_or(0.0, |percent| clamp_percent(*percent))
    }

    pub fn adapter(&self, luid: u64) -> AdapterUsage {
        let mut engines: Vec<GpuEngineLoad> = Vec::new();
        for ((engine_luid, _, _), total) in &self.engines {
            if *engine_luid != luid {
                continue;
            }
            let percent = clamp_percent(total.percent);
            match engines.iter_mut().find(|load| load.kind == total.engtype) {
                Some(load) => load.usage_percent = load.usage_percent.max(percent),
                None => engines.push(GpuEngineLoad {
                    kind: total.engtype.clone(),
                    usage_percent: percent,
                }),
            }
        }
        engines.sort_by(|a, b| {
            b.usage_percent
                .total_cmp(&a.usage_percent)
                .then_with(|| a.kind.cmp(&b.kind))
        });
        let usage_percent = engines.first().map_or(0.0, |load| load.usage_percent);
        AdapterUsage {
            usage_percent,
            engines,
        }
    }
}

pub fn clamp_percent(value: f64) -> f32 {
    value.clamp(0.0, 100.0) as f32
}
