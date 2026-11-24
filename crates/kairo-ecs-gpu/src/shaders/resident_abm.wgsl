struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
};

struct Uniforms {
    dt: f32,
    seed_lo: u32,
    seed_hi: u32,
    count: u32,
};

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

fn deterministic_jitter(seed: u32, index: u32) -> f32 {
    var state = seed ^ (index * 747796405u);
    state = (((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u);
    state = (state >> 22u) ^ state;
    return (f32(state & 0xffffu) / 65535.0) - 0.5;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= uniforms.count) {
        return;
    }
    let jitter = deterministic_jitter(uniforms.seed_lo, idx) * 0.001;
    particles[idx].x += (particles[idx].vx + jitter) * uniforms.dt;
    particles[idx].y += (particles[idx].vy - jitter) * uniforms.dt;
}
