// Path: crates/im_system_load/src/sampler.rs
// Description: Windows Sampler - owns the PDH query, adapter list and process state; assembles snapshots

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::file_description::LabelCache;
use crate::gpu_adapters::{self, GpuAdapter};
use crate::gpu_counters::GpuCounters;
use crate::pdh_query::PdhQuery;
use crate::process_groups::{GroupTable, ProcessLoad};
use crate::process_rates::ProcessRates;
use crate::process_table::ProcessTable;
use crate::snapshot::{CpuLoad, MemoryLoad, ProcessGroupLoad, SystemLoadSnapshot};
use crate::system_counters::SystemCounters;
use crate::{cpu_identity, memory_status, SystemLoadError};

const TOP_GROUPS: usize = 8;
/// The System Idle Process.
const IDLE_PID: u32 = 0;

pub struct Sampler {
    query: PdhQuery,
    system: SystemCounters,
    gpu: GpuCounters,
    adapters: Vec<GpuAdapter>,
    table: ProcessTable,
    rates: ProcessRates,
    groups: GroupTable,
    labels: LabelCache,
    cpu_name: Option<String>,
    logical_cpus: u32,
    last_sample: Option<Instant>,
}

impl Sampler {
    pub fn new() -> Result<Self, SystemLoadError> {
        let mut query = PdhQuery::open()?;
        let system = SystemCounters::add(&mut query)?;
        let gpu = GpuCounters::add(&mut query)?;
        Ok(Self {
            query,
            system,
            gpu,
            adapters: gpu_adapters::enumerate()?,
            table: ProcessTable::new(),
            rates: ProcessRates::default(),
            groups: GroupTable::default(),
            labels: LabelCache::default(),
            cpu_name: cpu_identity::processor_name(),
            logical_cpus: cpu_identity::logical_processor_count()?,
            last_sample: None,
        })
    }

    pub fn sample(&mut self) -> Result<SystemLoadSnapshot, SystemLoadError> {
        let now = Instant::now();
        let elapsed = self.last_sample.replace(now).map(|last| now - last);
        let elapsed_secs = elapsed.map(|d| d.as_secs_f64()).filter(|secs| *secs > 0.0);

        self.query.collect()?;
        self.gpu.read(&mut self.query)?;
        let mut core_percents = Vec::with_capacity(self.logical_cpus as usize);
        let cpu = self.system.cpu(&mut self.query, &mut core_percents)?;
        let disks = self.system.disks(&mut self.query)?;
        let networks = self.system.networks(&mut self.query)?;
        let hard_faults_per_sec = self.system.hard_faults_per_sec(&self.query);
        let memory = memory_status::read()?;

        let (process_count, thread_count) = self.collect_processes(elapsed_secs)?;
        let processes = self.top_groups();

        Ok(SystemLoadSnapshot {
            sampled_at_ms: unix_millis(),
            interval_ms: elapsed.map_or(0, |d| d.as_millis() as u64),
            cpu: CpuLoad {
                name: self.cpu_name.clone(),
                usage_percent: cpu.usage_percent,
                core_percents,
                speed_mhz: cpu.speed_mhz,
                base_speed_mhz: cpu.base_speed_mhz,
                process_count,
                thread_count,
            },
            memory: MemoryLoad {
                total_bytes: memory.total_bytes,
                available_bytes: memory.available_bytes,
                commit_bytes: memory.commit_bytes,
                commit_limit_bytes: memory.commit_limit_bytes,
                hard_faults_per_sec,
            },
            gpus: self.gpu.gpus(&self.adapters),
            disks,
            networks,
            processes,
        })
    }

    /// Feeds every live process into the rate tracker and group table.
    fn collect_processes(
        &mut self,
        elapsed_secs: Option<f64>,
    ) -> Result<(u32, u32), SystemLoadError> {
        let records = self.table.refresh()?;
        let (mut process_count, mut thread_count) = (0u32, 0u32);
        self.groups.begin();
        for record in records.iter().filter(|record| record.pid != IDLE_PID) {
            process_count += 1;
            thread_count += record.threads;
            let rate = self.rates.observe(
                (record.pid, record.create_time),
                record.cpu_time,
                record.io_bytes,
                elapsed_secs,
                self.logical_cpus,
            );
            let load = ProcessLoad {
                pid: record.pid,
                cpu_percent: rate.cpu_percent,
                memory_bytes: record.private_bytes,
                io_bytes_per_sec: rate.io_bytes_per_sec,
                gpu_percent: self.gpu.process_percent(record.pid),
                gpu_memory_bytes: self.gpu.process_memory_bytes(record.pid),
            };
            self.groups.add(&record.name, &load);
        }
        self.rates.finish();
        self.groups.finish();
        Ok((process_count, thread_count))
    }

    fn top_groups(&mut self) -> Vec<ProcessGroupLoad> {
        let labels = &mut self.labels;
        self.groups
            .top(TOP_GROUPS)
            .into_iter()
            .map(|(name, totals)| ProcessGroupLoad {
                name: name.to_owned(),
                label: labels.label(name, totals.pid),
                count: totals.count,
                cpu_percent: totals.cpu_percent,
                memory_bytes: totals.memory_bytes,
                io_bytes_per_sec: totals.io_bytes_per_sec,
                gpu_percent: totals.gpu_percent,
                gpu_memory_bytes: totals.gpu_memory_bytes,
            })
            .collect()
    }
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as u64)
}
