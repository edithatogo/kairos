use kairo_ecs_gpu::{
    AgentParticle, CpuFallbackParityContract, DesEvent, GpuBatchCommand, GpuBatchDispatch,
    GpuComputeError, GpuFallbackPolicy, GpuState, NativeBackendContractError, NativeGpuBackendKind,
    NativeGpuBackendRequest, PersistentDeviceMemory, PersistentGpuSession, ResidentBufferKind,
    CUDA_BACKEND_NOT_CONFIGURED, WGPU_BACKEND_NOT_CONFIGURED,
};

fn mixed_state() -> GpuState {
    GpuState {
        particles: vec![
            AgentParticle {
                x: 1.0,
                y: 2.0,
                vx: 0.5,
                vy: -0.25,
            },
            AgentParticle {
                x: -4.0,
                y: 3.5,
                vx: 1.25,
                vy: 0.75,
            },
        ],
        entity_values: vec![0, 10, -2],
    }
}

fn mixed_events() -> Vec<DesEvent> {
    vec![
        DesEvent {
            timestamp_ns: 30,
            entity_id: 2,
            delta: 4,
        },
        DesEvent {
            timestamp_ns: 10,
            entity_id: 1,
            delta: -3,
        },
        DesEvent {
            timestamp_ns: 20,
            entity_id: 1,
            delta: 8,
        },
    ]
}

#[test]
fn persistent_device_memory_declares_resident_buffers() {
    let state = mixed_state();
    let footprint = state.footprint().unwrap();
    let memory = PersistentDeviceMemory::from_state(&state).unwrap();

    assert_eq!(memory.total_bytes(), footprint.total_bytes());
    assert_eq!(memory.buffers().len(), 2);
    assert!(memory.contains(ResidentBufferKind::Particles));
    assert!(memory.contains(ResidentBufferKind::EntityValues));
    assert_eq!(
        memory
            .buffer(ResidentBufferKind::Particles)
            .unwrap()
            .len_bytes,
        footprint.particles_bytes
    );
}

#[test]
fn kernel_contract_names_dispatch_and_transient_uploads() {
    let state = mixed_state();
    let events = mixed_events();
    let abm = GpuBatchCommand::AbmStep { dt: 0.25, seed: 99 }
        .kernel_contract(&state)
        .unwrap();
    let des = GpuBatchCommand::DesStep {
        events: events.clone(),
    }
    .kernel_contract(&state)
    .unwrap();

    assert_eq!(abm.kernel_name, "abm_step");
    assert_eq!(
        abm.dispatch_shape.invocation_count,
        state.particles.len() as u32
    );
    assert_eq!(abm.transient_upload_bytes, 0);
    assert_eq!(des.kernel_name, "des_dispatch");
    assert_eq!(des.dispatch_shape.invocation_count, events.len() as u32);
    assert_eq!(
        des.transient_upload_bytes,
        core::mem::size_of_val(events.as_slice())
    );
}

#[test]
fn persistent_session_batch_dispatch_matches_cpu_fallback_contract() {
    let commands = vec![
        GpuBatchCommand::AbmStep { dt: 0.25, seed: 99 },
        GpuBatchCommand::DesStep {
            events: mixed_events(),
        },
    ];
    let contract = CpuFallbackParityContract::new(mixed_state(), commands);
    let mut session = PersistentGpuSession::new("contract-gpu");

    let report = contract.evaluate_persistent_session(&mut session).unwrap();

    assert_eq!(report.commands_checked, 2);
    assert!(report.state_matches);
    assert_eq!(report.cpu_stats.commands_dispatched, 2);
    assert_eq!(report.candidate_stats.commands_dispatched, 2);
    assert_eq!(
        report.cpu_stats.uploaded_bytes,
        report.candidate_stats.uploaded_bytes
    );
}

#[test]
fn persistent_batch_dispatch_requires_resident_state() {
    let mut session = PersistentGpuSession::new("contract-gpu");
    let commands = vec![GpuBatchCommand::AbmStep { dt: 0.1, seed: 1 }];

    assert_eq!(
        session.dispatch_batch(&commands),
        Err(GpuComputeError::StateNotResident)
    );
}

#[test]
fn backend_request_distinguishes_fallback_disabled_from_optional_unavailable() {
    let required = NativeGpuBackendRequest::required(NativeGpuBackendKind::Wgpu);
    let optional = NativeGpuBackendRequest::optional(NativeGpuBackendKind::Cuda);

    assert_eq!(
        required.fallback_policy,
        GpuFallbackPolicy::DisableCpuFallback
    );
    assert_eq!(
        required.unavailable_error("no adapter"),
        NativeBackendContractError::FallbackDisabled {
            backend: WGPU_BACKEND_NOT_CONFIGURED,
            reason: "no adapter"
        }
    );
    assert_eq!(
        optional.fallback_policy,
        GpuFallbackPolicy::AllowCpuFallback
    );
    assert_eq!(
        optional.unavailable_error("no context"),
        NativeBackendContractError::DeviceUnavailable {
            backend: CUDA_BACKEND_NOT_CONFIGURED,
            reason: "no context"
        }
    );
}
