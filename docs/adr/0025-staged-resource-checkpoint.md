# Staged resource checkpoints across processes

`kairo-ecs-des` exposes `StagedResourceRuntime::checkpoint_json` and
`StagedResourceRuntime::from_checkpoint_json` behind the opt-in
`checkpoint-json` feature. Capture the allocator at the same model event
boundary as the outer scheduler, model and RNG. Store its bytes in the
model payload of a [native engine checkpoint](0021-native-engine-checkpoint.md)
or another integrity-checked container.

Version 1 stores the existing scheduler checkpoint, allocated entity IDs,
resource quantities and membership order, request states and pending actions,
occupancy boundaries, provenance and the observation seal. The decoder
reconstructs available capacity and the live lease and action maps. It checks
pending action identity and ordering constraints, FIFO head eligibility, and
capacity across saved occupancy changes. A release and its FIFO promotions
can share one dispatch. The scheduler retains counters and live events, so
the application supplies trusted bytes for the matching run.

Save the `entity_id()` of each handle the model needs. After restart, bind
those IDs with `actor_by_entity`, `resource_by_entity` and
`request_by_entity`. Use `request` to retrieve the current lease. The
restore creates an independent allocator with its own handle namespace.
The `optimistic-usage` feature continues to provide in-process checkpoints
for rollback and requires a committed cutoff for occupancy export.

The HDF5 and ADIOS2 BP5 integration test launches separate reference, writer
and resumer processes. It compares the completed occupancy and capacity,
event dispatch order and identities, next RNG draw, and final allocator
checkpoint. Both schedulers in the fixture dispatch events above
`u64::MAX` ticks.
