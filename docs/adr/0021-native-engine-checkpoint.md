# Native engine checkpoints

The optional `engine-checkpoint` feature uses the Rust scheduler checkpoint
codec. With `hdf5` or `adios2`, `write_engine_checkpoint` publishes scheduler
state, caller-owned model bytes and event records in a single native
checkpoint. Capture those inputs at the same event boundary. The model owns
its component, resource, orchestration and RNG data and passes an exact
versioned schema identifier.

`restore_engine_checkpoint` validates the native layout and full digest before
decoding the scheduler, then returns the manifest, independent scheduler and
model bytes. Callers decode their model bytes before dispatch. The scheduler
codec verifies the clock, pending queue, cancellation counter and next
identities. The model decoder validates the state and domain-specific
relationship to the scheduler.

## Native layout and compatibility

V2 uses the v1 event and block representation plus a rank-one unsigned
byte dataset or variable named `engine_state`. Its first two little-endian
64-bit values give the lengths of the UTF-8 model schema and the existing
scheduler checkpoint JSON. The schema and scheduler bytes follow in that
order, with the model bytes in the remainder. The BP5 counts array adds the
length of `engine_state` as its seventh value. The SHA-256 covers the v2
schema marker, manifest, state length and every byte of engine state.

Use the v1 methods for telemetry checkpoints and the engine methods for v2.
Each reader checks its expected schema marker. The new methods reuse atomic
publication in their respective backends. Use a new destination when moving
between file versions. Use the application's checkpoint identity and
provenance to select a trusted checkpoint. The digest checks corruption.

## Conformance and review

The native restart suite launches three fresh processes for each backend.
They run an uninterrupted reference, save a partially executed service queue
and resume from that single native artifact. The suite compares next event
identities, equal-tick ordering, active jobs, timeouts, cancellation counts,
saved RNG position, the complete dispatched suffix and final totals. It
rejects a mismatched model schema and model bytes changed on disk.

The byte fields inherit each backend's rank, type and physical extent
validation. Checked lengths split the state payload before scheduler
decoding. The installed readers identify both the v2 schema and the native
`engine_state` field.
