#![cfg(feature = "cuda-backend")]

use kairo_ecs_gpu::{
    backends::cuda_backend::CudaBackend, AgentParticle, CpuFallbackCompute, DesEvent,
    GpuBackendAvailability, GpuCompute, GpuComputeError, GpuState,
};

#[test]
#[ignore = "requires a native NVIDIA CUDA device"]
fn native_model_interleaves_cuda_particle_and_des_dispatch() {
    let mut gpu = CudaBackend::initialize_hardware_only().unwrap();
    assert_eq!(gpu.availability(), GpuBackendAvailability::Available);
    assert!(matches!(
        gpu.download_state(),
        Err(GpuComputeError::StateNotResident)
    ));
    let mut initial = GpuState {
        particles: vec![
            AgentParticle {
                x: 2.0,
                y: 1.0,
                vx: 0.5,
                vy: -0.25
            };
            257
        ],
        entity_values: (0..65).map(|index| index * 7 - 26).collect(),
    };
    initial.entity_values[0] = i32::MIN;
    initial.entity_values[1] = i32::MAX;
    let mut cpu = CpuFallbackCompute::new();
    cpu.upload_state(&initial).unwrap();
    assert_eq!(
        gpu.upload_state(&initial).unwrap().uploaded_bytes,
        257 * 16 + 65 * 4
    );
    let limits = [
        DesEvent {
            timestamp_ns: 0,
            entity_id: 0,
            delta: -1,
        },
        DesEvent {
            timestamp_ns: 0,
            entity_id: 1,
            delta: 1,
        },
    ];
    cpu.run_des_step(&limits).unwrap();
    gpu.run_des_step(&limits).unwrap();
    for tick in 0..4u64 {
        let abm = gpu.run_abm_step(0.125, tick).unwrap();
        assert_eq!(abm.dispatched_workgroups, 2);
        assert_eq!((abm.uploaded_bytes, abm.downloaded_bytes), (0, 0));
        cpu.run_abm_step(0.125, tick).unwrap();
        let events: Vec<_> = (0..513)
            .map(|index| DesEvent {
                timestamp_ns: tick * 10_000 + index,
                entity_id: index * 7 % 65,
                delta: ((index + tick) % 11) as i32 - 5,
            })
            .collect();
        let des = gpu.run_des_step(&events).unwrap();
        assert_eq!(des.dispatched_workgroups, 3);
        assert_eq!((des.uploaded_bytes, des.downloaded_bytes), (513 * 8, 0));
        cpu.run_des_step(&events).unwrap();
    }
    let bad = [
        DesEvent {
            timestamp_ns: 0,
            entity_id: 0,
            delta: 99,
        },
        DesEvent {
            timestamp_ns: 1,
            entity_id: u64::MAX,
            delta: -1,
        },
    ];
    assert_eq!(
        gpu.run_des_step(&bad),
        Err(GpuComputeError::EntityOutOfRange {
            entity_id: u64::MAX,
            entity_count: 65
        })
    );
    let actual = gpu.download_state().unwrap();
    let expected = cpu.download_state().unwrap();
    assert_eq!(actual.entity_values, expected.entity_values);
    assert_eq!(actual.particles.len(), expected.particles.len());
    for (a, e) in actual.particles.iter().zip(&expected.particles) {
        assert!((a.x - e.x).abs().max((a.y - e.y).abs()) < 2e-6);
        assert_eq!((a.vx, a.vy), (e.vx, e.vy));
    }
    let empty = GpuState::default();
    gpu.upload_state(&empty).unwrap();
    assert_eq!(gpu.download_state().unwrap(), empty);
}
