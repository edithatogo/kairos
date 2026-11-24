struct Event {
    entity: u32,
    delta: i32,
};

@group(0) @binding(0) var<storage, read_write> values: array<atomic<i32>>;
@group(0) @binding(1) var<storage, read> events: array<Event>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&events)) {
        let event = events[id.x];
        atomicAdd(&values[event.entity], event.delta);
    }
}
