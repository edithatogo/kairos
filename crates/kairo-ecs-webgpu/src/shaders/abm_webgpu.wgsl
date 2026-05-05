struct Agent {
  position: vec2<f32>,
  velocity: vec2<f32>,
};

struct Params {
  dt: f32,
  seed: u32,
  count: u32,
  _pad: u32,
};

@group(0) @binding(0)
var<storage, read_write> agents: array<Agent>;

@group(0) @binding(1)
var<uniform> params: Params;

fn pcg_jitter(seed: u32, index: u32) -> f32 {
  var state = seed ^ (index * 747796405u);
  state = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
  state = (state >> 22u) ^ state;
  return (f32(state & 65535u) / 65535.0) - 0.5;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let index = id.x;
  if (index >= params.count) {
    return;
  }

  let jitter = pcg_jitter(params.seed, index) * 0.001;
  agents[index].position.x = agents[index].position.x + (agents[index].velocity.x + jitter) * params.dt;
  agents[index].position.y = agents[index].position.y + (agents[index].velocity.y - jitter) * params.dt;
}
