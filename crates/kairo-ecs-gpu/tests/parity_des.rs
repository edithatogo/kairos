use kairo_ecs_gpu::{CpuFallbackCompute, DesEvent, GpuCompute, GpuState};

#[test]
fn fixed_seed_des_parity_harness_applies_events_in_timestamp_order() {
    let state = GpuState {
        particles: vec![],
        entity_values: vec![0, 0, 0],
    };
    let events = vec![
        DesEvent {
            timestamp_ns: 20,
            entity_id: 1,
            delta: 7,
        },
        DesEvent {
            timestamp_ns: 10,
            entity_id: 1,
            delta: -2,
        },
        DesEvent {
            timestamp_ns: 10,
            entity_id: 2,
            delta: 4,
        },
    ];

    let mut cpu = CpuFallbackCompute::new();
    let mut gpu_contract = CpuFallbackCompute::new();

    cpu.upload_state(&state).unwrap();
    gpu_contract.upload_state(&state).unwrap();
    cpu.run_des_step(&events).unwrap();
    gpu_contract.run_des_step(&events).unwrap();

    assert_eq!(
        cpu.download_state().unwrap(),
        gpu_contract.download_state().unwrap()
    );
}
