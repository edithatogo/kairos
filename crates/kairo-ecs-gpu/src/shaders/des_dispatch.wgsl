struct Event {
  timestamp_low: u32,
  timestamp_high: u32,
  entity_id_low: u32,
  entity_id_high: u32,
  delta: i32,
  _pad0: u32,
};

struct Params {
  event_count: u32,
  entity_count: u32,
  _pad0: u32,
  _pad1: u32,
};

@group(0) @binding(0)
var<storage, read> events: array<Event>;

@group(0) @binding(1)
var<storage, read_write> values: array<atomic<i32>>;

@group(0) @binding(2)
var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  if (id.x >= params.event_count) {
    return;
  }

  let event = events[id.x];
  if (event.entity_id_high != 0u || event.entity_id_low >= params.entity_count) {
    return;
  }

  atomicAdd(&values[event.entity_id_low], event.delta);
}
