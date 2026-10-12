// Path: crates/im_system_load/src/gpu_usage_tests.rs
// Description: Tests for the Task Manager GPU aggregation rule

use crate::gpu_instance::parse_engine;
use crate::gpu_usage::GpuUsage;

fn collect(usage: &mut GpuUsage, samples: &[(&str, f64)]) {
    usage.begin();
    for (name, percent) in samples {
        if let Some(instance) = parse_engine(name) {
            usage.add(&instance, *percent);
        }
    }
    usage.finish();
}

const A: &str = "luid_0x00000000_0x0000D1B5";
const B: &str = "luid_0x00000000_0x0000E000";

fn engine(pid: u32, luid: &str, eng: u32, kind: &str) -> String {
    format!("pid_{pid}_{luid}_phys_0_eng_{eng}_engtype_{kind}")
}

#[test]
fn sums_processes_per_engine_and_reports_busiest_engine() {
    let names = [
        engine(10, A, 0, "3D"),
        engine(20, A, 0, "3D"),
        engine(10, A, 4, "Copy"),
        engine(30, A, 5, "Copy"),
        engine(30, A, 6, "Copy"),
        engine(40, B, 0, "3D"),
    ];
    let samples: Vec<(&str, f64)> = names
        .iter()
        .map(String::as_str)
        .zip([30.0, 25.0, 10.0, 20.0, 40.0, 90.0])
        .collect();
    let mut usage = GpuUsage::default();
    collect(&mut usage, &samples);

    let a = usage.adapter(crate::gpu_instance::luid_key(0, 0xD1B5));
    assert_eq!(a.usage_percent, 55.0);
    let kinds: Vec<(&str, f32)> = a
        .engines
        .iter()
        .map(|load| (load.kind.as_str(), load.usage_percent))
        .collect();
    assert_eq!(kinds, vec![("3D", 55.0), ("Copy", 40.0)]);

    assert_eq!(usage.process_percent(10), 30.0);
    assert_eq!(usage.process_percent(30), 40.0);
    assert_eq!(usage.process_percent(99), 0.0);
}

#[test]
fn clamps_engine_sums_and_forgets_idle_engines() {
    let busy = [engine(1, A, 0, "3D"), engine(2, A, 0, "3D")];
    let mut usage = GpuUsage::default();
    collect(
        &mut usage,
        &[(busy[0].as_str(), 70.0), (busy[1].as_str(), 60.0)],
    );
    let luid = crate::gpu_instance::luid_key(0, 0xD1B5);
    assert_eq!(usage.adapter(luid).usage_percent, 100.0);

    collect(&mut usage, &[]);
    let idle = usage.adapter(luid);
    assert_eq!(idle.usage_percent, 0.0);
    assert!(idle.engines.is_empty());
    assert_eq!(usage.process_percent(1), 0.0);
}
