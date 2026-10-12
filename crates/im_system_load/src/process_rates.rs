// Path: crates/im_system_load/src/process_rates.rs
// Description: Per-process CPU share and I/O rate from deltas keyed by (pid, create time)

use std::collections::HashMap;

/// 100 ns ticks per second, the unit of process CPU times.
const TICKS_PER_SEC: f64 = 10_000_000.0;

type ProcessKey = (u32, i64);

#[derive(Clone, Copy)]
struct Totals {
    cpu_time: u64,
    io_bytes: u64,
}

/// Holds the previous sample's totals; processes absent from a sample are dropped.
#[derive(Default)]
pub struct ProcessRates {
    previous: HashMap<ProcessKey, Totals>,
    current: HashMap<ProcessKey, Totals>,
}

pub struct ProcessRate {
    pub cpu_percent: f32,
    pub io_bytes_per_sec: f64,
}

impl ProcessRates {
    /// `elapsed_secs` is `None` on the first sample, which reports zero rates.
    pub fn observe(
        &mut self,
        key: (u32, i64),
        cpu_time: u64,
        io_bytes: u64,
        elapsed_secs: Option<f64>,
        logical_cpus: u32,
    ) -> ProcessRate {
        self.current.insert(key, Totals { cpu_time, io_bytes });
        let (Some(secs), Some(previous)) = (elapsed_secs, self.previous.get(&key)) else {
            return ProcessRate {
                cpu_percent: 0.0,
                io_bytes_per_sec: 0.0,
            };
        };
        let cpu_ticks = cpu_time.saturating_sub(previous.cpu_time) as f64;
        let capacity = secs * TICKS_PER_SEC * f64::from(logical_cpus.max(1));
        ProcessRate {
            cpu_percent: (cpu_ticks / capacity * 100.0).clamp(0.0, 100.0) as f32,
            io_bytes_per_sec: io_bytes.saturating_sub(previous.io_bytes) as f64 / secs,
        }
    }

    pub fn finish(&mut self) {
        std::mem::swap(&mut self.previous, &mut self.current);
        self.current.clear();
    }
}
