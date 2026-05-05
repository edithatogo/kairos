use kairo_ecs_gpu::{AgentParticle, CpuFallbackCompute, GpuCompute, GpuState};

#[test]
fn fixed_seed_abm_parity_harness_is_deterministic() {
    let state = GpuState {
        particles: (0..512)
            .map(|i| AgentParticle {
                x: i as f32,
                y: (i as f32) * 0.5,
                vx: 0.25,
                vy: -0.125,
            })
            .collect(),
        entity_values: vec![],
    };

    let mut cpu = CpuFallbackCompute::new();
    let mut gpu_contract = CpuFallbackCompute::new();

    cpu.upload_state(&state).unwrap();
    gpu_contract.upload_state(&state).unwrap();

    cpu.run_abm_step(0.016, 12_345).unwrap();
    gpu_contract.run_abm_step(0.016, 12_345).unwrap();

    assert_eq!(
        cpu.download_state().unwrap(),
        gpu_contract.download_state().unwrap()
    );
}
