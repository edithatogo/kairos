use kairo_ecs_gpu::{
    AgentParticle, CpuFallbackCompute, DesEvent, DispatchShape, GpuCompute, GpuMemoryBudget,
    GpuState, GpuWorkloadKind, DEFAULT_WORKGROUP_SIZE, TRACK32_TARGET_MEMORY_BUDGET,
};

#[test]
fn cpu_fallback_reports_explicit_capabilities() {
    let backend = CpuFallbackCompute::new();
    let capabilities = backend.capabilities();

    assert_eq!(capabilities.backend_name, "cpu-fallback");
    assert_eq!(capabilities.max_workgroup_size, DEFAULT_WORKGROUP_SIZE);
    assert!(!capabilities.supports_zero_copy_borrow);
    assert!(!capabilities.supports_unified_memory);
}

#[test]
fn footprint_and_dispatch_contract_are_hardware_independent() {
    let state = GpuState {
        particles: vec![
            AgentParticle {
                x: 0.0,
                y: 0.0,
                vx: 1.0,
                vy: -1.0,
            };
            513
        ],
        entity_values: vec![0; 32],
    };

    let footprint = state.footprint().unwrap();
    let dispatch = DispatchShape::for_items(state.particles.len()).unwrap();

    assert!(footprint.fits_within(TRACK32_TARGET_MEMORY_BUDGET));
    assert_eq!(dispatch.workgroup_size, 256);
    assert_eq!(dispatch.workgroup_count, 3);
    assert_eq!(dispatch.invocation_count, 513);
}

#[test]
fn execution_plans_are_public_and_backend_agnostic() {
    let state = GpuState {
        particles: vec![AgentParticle {
            x: 1.0,
            y: 2.0,
            vx: 0.5,
            vy: -0.25,
        }],
        entity_values: vec![3, 4],
    };

    let plan = state.abm_execution_plan(0.5, 7).unwrap();

    assert_eq!(plan.workload, GpuWorkloadKind::AbmStep { dt: 0.5, seed: 7 });
    assert!(plan.fits_within_budget());
    assert_eq!(plan.transfer_bytes_to_gpu(), plan.transfer_bytes_from_gpu());
}

#[test]
fn des_execution_plan_budget_api_includes_event_buffers() {
    let state = GpuState {
        particles: vec![],
        entity_values: vec![0, 0],
    };
    let events = vec![
        DesEvent {
            timestamp_ns: 20,
            entity_id: 1,
            delta: 3,
        },
        DesEvent {
            timestamp_ns: 10,
            entity_id: 0,
            delta: 5,
        },
    ];

    let mut plan = state.des_execution_plan(&events).unwrap();
    let state_bytes = state.footprint().unwrap().total_bytes();
    let event_bytes = core::mem::size_of_val(&events[..]);

    assert_eq!(
        plan.workload,
        GpuWorkloadKind::DesStep {
            event_count: events.len(),
            event_bytes,
        }
    );
    assert_eq!(
        plan.checked_device_bytes_required().unwrap(),
        state_bytes + event_bytes
    );
    assert_eq!(
        plan.checked_staging_bytes_required().unwrap(),
        2 * (state_bytes + event_bytes)
    );

    plan.memory_budget = GpuMemoryBudget::new(state_bytes + event_bytes, 2 * state_bytes);
    assert!(!plan.fits_within_budget());
}
