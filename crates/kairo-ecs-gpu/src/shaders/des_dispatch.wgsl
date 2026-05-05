struct Event {
  timestamp_low: u32,
  timestamp_high: u32,
  entity_id: u32,
  delta: i32,
};

@group(0) @binding(0)
var<storage, read> events: array<Event>;

@group(0) @binding(1)
var<storage, read_write> values: array<atomic<i32>>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let event = events[id.x];
  atomicAdd(&values[event.entity_id], event.delta);
}
