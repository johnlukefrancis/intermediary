// Path: crates/im_system_load/src/gpu_instance_tests.rs
// Description: Tests for PDH GPU instance-name parsing

use crate::gpu_instance::{
    luid_key, parse_adapter_memory, parse_engine, parse_process_memory, AdapterMemoryInstance,
    EngineInstance, ProcessMemoryInstance,
};

#[test]
fn parses_engine_instance() {
    let parsed = parse_engine("pid_1234_luid_0x00000000_0x0000D1B5_phys_0_eng_3_engtype_3D");
    assert_eq!(
        parsed,
        Some(EngineInstance {
            pid: 1234,
            luid: luid_key(0, 0xD1B5),
            phys: 0,
            eng: 3,
            engtype: "3D",
        })
    );
}

#[test]
fn engine_type_keeps_its_underscores() {
    let parsed = parse_engine("pid_8_luid_0x00000001_0x0000A000_phys_1_eng_12_engtype_Compute_0");
    let parsed = parsed.map(|instance| (instance.luid, instance.phys, instance.engtype));
    assert_eq!(parsed, Some((luid_key(1, 0xA000), 1, "Compute_0")));
}

#[test]
fn parses_process_memory_instance() {
    assert_eq!(
        parse_process_memory("pid_1234_luid_0x00000000_0x0000D1B5_phys_0"),
        Some(ProcessMemoryInstance {
            pid: 1234,
            luid: luid_key(0, 0xD1B5),
        })
    );
}

#[test]
fn parses_adapter_memory_instance() {
    assert_eq!(
        parse_adapter_memory("luid_0x00000000_0x0000D1B5_phys_0"),
        Some(AdapterMemoryInstance {
            luid: luid_key(0, 0xD1B5),
            phys: 0,
        })
    );
}

#[test]
fn rejects_foreign_shapes() {
    assert_eq!(
        parse_engine("pid_1234_luid_0x00000000_0x0000D1B5_phys_0"),
        None
    );
    assert_eq!(
        parse_engine("pid_x_luid_0x0_0x1_phys_0_eng_0_engtype_3D"),
        None
    );
    assert_eq!(
        parse_engine("pid_1_luid_0x0_0x1_phys_0_eng_0_engtype_"),
        None
    );
    assert_eq!(
        parse_process_memory("luid_0x00000000_0x0000D1B5_phys_0"),
        None
    );
    assert_eq!(
        parse_process_memory("pid_1_luid_0x0_0x1_phys_0_eng_0"),
        None
    );
    assert_eq!(
        parse_adapter_memory("luid_00000000_0x0000D1B5_phys_0"),
        None
    );
    assert_eq!(parse_adapter_memory("_Total"), None);
}
