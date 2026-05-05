use kairo_ecs_webgpu::{run_webgpu_step, AgentSnapshot};

#[test]
fn fixed_seed_webgpu_contract_matches_cpu_reference() {
    let mut cpu = seed_agents();
    let mut webgpu_contract = seed_agents();

    run_cpu_step(&mut cpu, 0.016, 99);
    run_webgpu_step(&mut webgpu_contract, 0.016, 99);

    assert_eq!(cpu, webgpu_contract);
}

fn seed_agents() -> Vec<AgentSnapshot> {
    (0..128)
        .map(|i| AgentSnapshot {
            x: i as f32,
            y: (i as f32) * 0.25,
            vx: 0.75,
            vy: -0.5,
        })
        .collect()
}

fn run_cpu_step(agents: &mut [AgentSnapshot], dt: f32, seed: u32) {
    for (index, agent) in agents.iter_mut().enumerate() {
        let jitter = pcg_jitter(seed, index as u32) * 0.001;
        agent.x += (agent.vx + jitter) * dt;
        agent.y += (agent.vy - jitter) * dt;
    }
}

fn pcg_jitter(seed: u32, index: u32) -> f32 {
    let mut state = seed ^ index.wrapping_mul(747_796_405);
    state = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    state = (state >> 22) ^ state;
    ((state & 65_535) as f32 / 65_535.0) - 0.5
}
