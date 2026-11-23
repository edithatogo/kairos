#![cfg(all(target_os = "macos", feature = "wgpu-backend"))]

use kairo_ecs_gpu::{
    DesEvent, GpuBackendAvailability, GpuCompute, GpuComputeError, GpuState, GpuStepStats,
    MetalBackend,
};

#[test]
fn native_entities_stay_resident_across_batches_and_fail_closed() {
    let mut gpu = MetalBackend::initialize_macos_hardware_only().expect("Apple Metal");
    assert_eq!(
        gpu.capabilities().availability,
        GpuBackendAvailability::Available
    );
    assert!(matches!(
        gpu.run_des_step(&[]),
        Err(GpuComputeError::StateNotResident)
    ));

    let mut initial = GpuState {
        particles: Vec::new(),
        entity_values: (0..65).map(|index| index * 7 - 93).collect(),
    };
    initial.entity_values[0] = i32::MIN;
    initial.entity_values[7] = i32::MAX;
    let upload = gpu.upload_state(&initial).unwrap();
    assert_eq!(upload.uploaded_bytes, 65 * size_of::<i32>());
    let mut expected = initial.entity_values.clone();
    let wrap = [
        DesEvent {
            timestamp_ns: 0,
            entity_id: 0,
            delta: -1,
        },
        DesEvent {
            timestamp_ns: 0,
            entity_id: 7,
            delta: 1,
        },
    ];
    gpu.run_des_step(&wrap).unwrap();
    expected[0] = i32::MAX;
    expected[7] = i32::MIN;
    for batch in 0..4u64 {
        let events: Vec<_> = (0..131u64)
            .rev()
            .map(|index| DesEvent {
                timestamp_ns: batch * 1000 + index,
                entity_id: index * 7 % 65,
                delta: ((index + batch) % 11) as i32 - 5,
            })
            .collect();
        for event in &events {
            let value = &mut expected[event.entity_id as usize];
            *value = (i64::from(*value) + i64::from(event.delta)) as i32;
        }
        let stats = gpu.run_des_step(&events).unwrap();
        assert_eq!(stats.dispatched_workgroups, 3);
        assert_eq!(stats.uploaded_bytes, 131 * 8);
        assert_eq!(stats.downloaded_bytes, 0);
    }

    let bad = [
        DesEvent {
            timestamp_ns: 5000,
            entity_id: 0,
            delta: 100,
        },
        DesEvent {
            timestamp_ns: 5001,
            entity_id: u64::MAX,
            delta: 9,
        },
    ];
    assert_eq!(
        gpu.run_des_step(&bad),
        Err(GpuComputeError::EntityOutOfRange {
            entity_id: u64::MAX,
            entity_count: 65
        })
    );
    let after = gpu.download_state().unwrap();
    assert_eq!(after.entity_values, expected);
    let mut snapshot = gpu.clone();
    assert_eq!(
        snapshot.capabilities().availability,
        GpuBackendAvailability::CpuFallback
    );
    assert_eq!(snapshot.download_state().unwrap(), after);
    snapshot.run_des_step(&bad[..1]).unwrap();
    assert_ne!(snapshot.download_state().unwrap().entity_values, expected);
    assert_eq!(gpu.download_state().unwrap(), after);

    let empty = GpuState::default();
    assert_eq!(gpu.upload_state(&empty).unwrap().uploaded_bytes, 0);
    assert_eq!(gpu.run_des_step(&[]).unwrap(), GpuStepStats::default());
    assert_eq!(gpu.download_state().unwrap(), empty);
}
