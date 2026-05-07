# KairoECS Conformance Fixtures

This directory owns the shared behavioral fixtures consumed by Rust tests and later by Python, R, Julia, TypeScript, C#, and Go bindings.

The bootstrap contract is:

- Flat JSON fixtures live in `conformance/fixtures/`.
- `conformance/fixtures/manifest.json` records which fixtures are ready and who consumes them.
- `conformance/fixtures/README.md` defines the fields and assertions each fixture family must keep stable.

The longer-form directory-backed shape described in `conductor/contracts/conformance-contract.md` remains the target runner contract. These bootstrap files are the concrete inputs for the current implementation wave.

Current ready fixture IDs:

1. `scheduler_ordering_v1`
2. `scheduler_cancellation_v1`
3. `rng_reproducibility_v1`
4. `vvuq_scenario_replay_v1`
5. `zero_delay_guard_v1`

Current planned fixture families that are still future scope:

1. `des_resource_queue_v1`
2. `abm_behavior_update_v1`
3. `hybrid_des_abm_v1`
4. `arrow_event_log_v1`
5. `ffi_lifecycle_v1`

The fixture family semantics are:

- `scheduler_ordering_v1` covers deterministic event ordering by `(time, priority, sequence)`.
- `scheduler_cancellation_v1` covers cancellation without reordering remaining events.
- `rng_reproducibility_v1` covers reproducible entity-derived RNG streams.
- `vvuq_scenario_replay_v1` covers scenario/seed replay evidence boundaries for VVUQ and experiment-runner smoke checks.
- `zero_delay_guard_v1` covers zero-delay event ordering guardrails without claiming a native livelock harness.

The metadata-only chaos manifest lives at `conformance/chaos/manifest.json` and
is validated by `tests/conformance/chaos-check.mjs`. It covers event
corruption, entity exhaustion, telemetry loss, and ordering inversion without
requiring native link tests.

Downstream tracks should treat these fixtures as the source of truth for core behavior. Track 01 consumes scheduler ordering, scheduler cancellation, and RNG fixtures; Track 02 consumes the planned FFI lifecycle fixture once it is ready; and Tracks 06-11 consume the same manifest without redefining the semantics locally.
