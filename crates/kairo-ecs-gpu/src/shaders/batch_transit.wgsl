// WGSL Compute Shader: batch_transit.wgsl
// Workgroup size: 256
// Computes Euclidean distance, transit velocity scaling, and arrival time for a batch of agents.

struct TransitQuery {
    start_x: f32,
    start_y: f32,
    dest_x: f32,
    dest_y: f32,
    speed: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

struct TransitResult {
    distance: f32,
    time_to_dest: f32,
    velocity_scale: f32,
    arrived: u32,
};

struct TransitParams {
    agent_count: u32,
    dt: f32,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0)
var<storage, read> queries: array<TransitQuery>;

@group(0) @binding(1)
var<storage, read_write> results: array<TransitResult>;

@group(0) @binding(2)
var<uniform> params: TransitParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    if (index >= params.agent_count) {
        return;
    }

    let q = queries[index];

    // Enforce layout bounds clamping
    let sx = clamp(q.start_x, params.min_x, params.max_x);
    let sy = clamp(q.start_y, params.min_y, params.max_y);
    let dx = clamp(q.dest_x, params.min_x, params.max_x);
    let dy = clamp(q.dest_y, params.min_y, params.max_y);

    let diff_x = dx - sx;
    let diff_y = dy - sy;
    let distance = sqrt(diff_x * diff_x + diff_y * diff_y);

    var vel_scale: f32 = 1.0;
    var time_to_dest: f32 = 0.0;
    var arrived: u32 = 0u;

    if (distance <= 1e-6) {
        vel_scale = 0.0;
        time_to_dest = 0.0;
        arrived = 1u;
    } else if (q.speed > 0.0) {
        vel_scale = 1.0;
        time_to_dest = distance / q.speed;
        let step_advance = q.speed * params.dt;
        if (distance <= step_advance) {
            arrived = 1u;
        }
    } else {
        vel_scale = 0.0;
        time_to_dest = 1e30;
        arrived = 0u;
    }

    results[index].distance = distance;
    results[index].time_to_dest = time_to_dest;
    results[index].velocity_scale = vel_scale;
    results[index].arrived = arrived;
}
