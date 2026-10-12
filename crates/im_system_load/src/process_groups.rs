// Path: crates/im_system_load/src/process_groups.rs
// Description: Groups processes by image name and selects the union of the top groups per resource

use std::collections::HashMap;

/// One process's contribution to its group for this sample.
pub struct ProcessLoad {
    pub pid: u32,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub io_bytes_per_sec: f64,
    pub gpu_percent: f32,
    pub gpu_memory_bytes: u64,
}

#[derive(Default)]
pub struct GroupTotals {
    /// Any member pid, used for the executable-description lookup.
    pub pid: u32,
    pub count: u32,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub io_bytes_per_sec: f64,
    pub gpu_percent: f32,
    pub gpu_memory_bytes: u64,
}

/// Group entries persist across samples so steady-state sampling allocates no keys.
#[derive(Default)]
pub struct GroupTable {
    groups: HashMap<String, GroupTotals>,
}

impl GroupTable {
    pub fn begin(&mut self) {
        for totals in self.groups.values_mut() {
            *totals = GroupTotals::default();
        }
    }

    pub fn add(&mut self, name: &str, load: &ProcessLoad) {
        let totals = match self.groups.get_mut(name) {
            Some(totals) => totals,
            None => self.groups.entry(name.to_owned()).or_default(),
        };
        if totals.count == 0 {
            totals.pid = load.pid;
        }
        totals.count += 1;
        totals.cpu_percent = (totals.cpu_percent + load.cpu_percent).min(100.0);
        totals.memory_bytes += load.memory_bytes;
        totals.io_bytes_per_sec += load.io_bytes_per_sec;
        totals.gpu_percent = totals.gpu_percent.max(load.gpu_percent);
        totals.gpu_memory_bytes += load.gpu_memory_bytes;
    }

    /// Drops groups with no live process in this sample.
    pub fn finish(&mut self) {
        self.groups.retain(|_, totals| totals.count > 0);
    }

    /// Union of the top `limit` non-zero groups by CPU, memory, I/O, GPU and GPU memory.
    pub fn top(&self, limit: usize) -> Vec<(&str, &GroupTotals)> {
        let entries: Vec<(&str, &GroupTotals)> = self
            .groups
            .iter()
            .map(|(name, totals)| (name.as_str(), totals))
            .collect();
        let mut chosen = vec![false; entries.len()];
        let mut order: Vec<usize> = (0..entries.len()).collect();
        let metrics: [fn(&GroupTotals) -> f64; 5] = [
            |t| f64::from(t.cpu_percent),
            |t| t.memory_bytes as f64,
            |t| t.io_bytes_per_sec,
            |t| f64::from(t.gpu_percent),
            |t| t.gpu_memory_bytes as f64,
        ];
        for metric in metrics {
            order.sort_unstable_by(|a, b| metric(entries[*b].1).total_cmp(&metric(entries[*a].1)));
            for index in order.iter().take(limit) {
                if metric(entries[*index].1) > 0.0 {
                    chosen[*index] = true;
                }
            }
        }
        entries
            .into_iter()
            .zip(chosen)
            .filter_map(|(entry, keep)| keep.then_some(entry))
            .collect()
    }
}
