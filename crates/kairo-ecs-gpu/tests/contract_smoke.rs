use kairo_ecs_gpu::{
    AgentParticle, CpuFallbackCompute, DispatchShape, GpuCompute, GpuState, DEFAULT_WORKGROUP_SIZE,
    TRACK32_TARGET_MEMORY_BUDGET,
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
