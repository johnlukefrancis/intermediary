// Path: crates/im_system_load/src/system_counters.rs
// Description: PDH counters for CPU utility and speed, paging, physical disks and network interfaces

use crate::gpu_usage::clamp_percent;
use crate::pdh_query::{Counter, PdhQuery};
use crate::snapshot::{DiskLoad, NetworkLoad};
use crate::SystemLoadError;

pub struct CpuReading {
    pub usage_percent: f32,
    pub speed_mhz: Option<f32>,
    pub base_speed_mhz: Option<f32>,
}

pub struct SystemCounters {
    utility: Counter,
    frequency: Counter,
    performance: Counter,
    pages_input: Counter,
    disk_idle: Counter,
    disk_read: Counter,
    disk_write: Counter,
    net_received: Counter,
    net_sent: Counter,
    net_bandwidth: Counter,
    cores: Vec<((u32, u32), f32)>,
}

impl SystemCounters {
    pub fn add(query: &mut PdhQuery) -> Result<Self, SystemLoadError> {
        Ok(Self {
            utility: query.add(r"\Processor Information(*)\% Processor Utility")?,
            frequency: query.add(r"\Processor Information(_Total)\Processor Frequency")?,
            performance: query.add(r"\Processor Information(_Total)\% Processor Performance")?,
            pages_input: query.add(r"\Memory\Pages Input/sec")?,
            disk_idle: query.add(r"\PhysicalDisk(*)\% Idle Time")?,
            disk_read: query.add(r"\PhysicalDisk(*)\Disk Read Bytes/sec")?,
            disk_write: query.add(r"\PhysicalDisk(*)\Disk Write Bytes/sec")?,
            net_received: query.add(r"\Network Interface(*)\Bytes Received/sec")?,
            net_sent: query.add(r"\Network Interface(*)\Bytes Sent/sec")?,
            net_bandwidth: query.add(r"\Network Interface(*)\Current Bandwidth")?,
            cores: Vec::new(),
        })
    }

    /// Fills `core_percents` in processor order and returns the machine totals.
    pub fn cpu(
        &mut self,
        query: &mut PdhQuery,
        core_percents: &mut Vec<f32>,
    ) -> Result<CpuReading, SystemLoadError> {
        let mut usage_percent = 0.0;
        let cores = &mut self.cores;
        cores.clear();
        query.for_each(self.utility, |name, value| {
            if name == "_Total" {
                usage_percent = clamp_percent(value);
            } else if let Some(index) = core_index(name) {
                cores.push((index, clamp_percent(value)));
            }
        })?;
        cores.sort_unstable_by_key(|(index, _)| *index);
        core_percents.clear();
        core_percents.extend(cores.iter().map(|(_, percent)| *percent));

        let base = query.value(self.frequency).filter(|mhz| *mhz > 0.0);
        let speed = base
            .zip(query.value(self.performance))
            .map(|(mhz, performance)| (mhz * performance / 100.0) as f32);
        Ok(CpuReading {
            usage_percent,
            speed_mhz: speed,
            base_speed_mhz: base.map(|mhz| mhz as f32),
        })
    }

    pub fn hard_faults_per_sec(&self, query: &PdhQuery) -> f32 {
        query
            .value(self.pages_input)
            .map_or(0.0, |rate| rate.max(0.0) as f32)
    }

    pub fn disks(&self, query: &mut PdhQuery) -> Result<Vec<DiskLoad>, SystemLoadError> {
        let mut disks: Vec<DiskLoad> = Vec::new();
        query.for_each(self.disk_idle, |name, idle| {
            if name != "_Total" {
                disks.push(DiskLoad {
                    name: name.to_owned(),
                    active_percent: clamp_percent(100.0 - idle),
                    read_bytes_per_sec: 0.0,
                    write_bytes_per_sec: 0.0,
                });
            }
        })?;
        query.for_each(self.disk_read, |name, rate| {
            if let Some(disk) = disks.iter_mut().find(|disk| disk.name == name) {
                disk.read_bytes_per_sec = rate.max(0.0);
            }
        })?;
        query.for_each(self.disk_write, |name, rate| {
            if let Some(disk) = disks.iter_mut().find(|disk| disk.name == name) {
                disk.write_bytes_per_sec = rate.max(0.0);
            }
        })?;
        Ok(disks)
    }

    pub fn networks(&self, query: &mut PdhQuery) -> Result<Vec<NetworkLoad>, SystemLoadError> {
        let mut networks: Vec<NetworkLoad> = Vec::new();
        query.for_each(self.net_bandwidth, |name, bits| {
            networks.push(NetworkLoad {
                name: name.to_owned(),
                received_bytes_per_sec: 0.0,
                sent_bytes_per_sec: 0.0,
                link_bits_per_sec: bits.max(0.0),
            });
        })?;
        query.for_each(self.net_received, |name, rate| {
            if let Some(network) = networks.iter_mut().find(|network| network.name == name) {
                network.received_bytes_per_sec = rate.max(0.0);
            }
        })?;
        query.for_each(self.net_sent, |name, rate| {
            if let Some(network) = networks.iter_mut().find(|network| network.name == name) {
                network.sent_bytes_per_sec = rate.max(0.0);
            }
        })?;
        Ok(networks)
    }
}

/// `"<group>,<processor>"`; group totals such as `"0,_Total"` yield `None`.
fn core_index(name: &str) -> Option<(u32, u32)> {
    let (group, processor) = name.split_once(',')?;
    Some((group.parse().ok()?, processor.parse().ok()?))
}
