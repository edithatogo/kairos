# 12 Conformance, Testing & Benchmarks — spec.md

## Mission

Create the shared correctness, performance, fuzz, and cross-language fixture suite that every binding must satisfy.

## Primary subagent

```text
conformance-agent + performance-agent
```

## Dependencies

```text
Track 01 defines deterministic core behavior.
Track 02 exposes the stable facade used by binding runners.
Track 04 defines the Arrow schema fingerprint consumed by the event-log fixtures.
```

## Owned paths

```text
conformance, tests/conformance, benches, crates/kairo-ecs-bench
```

## Blocked paths

```text
crates/kairo-ecs-core/ — owned by Track 01 (scheduler implementation)
crates/kairo-ecs-ffi/ — owned by Track 02 (FFI bridge)
crates/kairo-ecs-des/ — owned by Track 03 (DES API)
crates/kairo-ecs-abm/ — owned by Track 03 (ABM API)
bindings/ — owned by Tracks 06-11
```

## Parallel-safe with

Most tracks are parallel-safe after their contract inputs are accepted. See `conductor/parallel-execution.md` for the wave model.

## Inputs

- Accepted project identity and naming status where relevant.
- Relevant files under `conductor/contracts/`.
- Prior track handoff notes.

## Outputs

- Implementation in owned paths exists and is wired to the workspace.
- Tests or test-plan.
- Docs updates.
- Release notes or compatibility notes when public surfaces change.

## Fixture strategy

Track 12 owns the shared fixture contract and keeps the fixture IDs stable for Track 01, Track 02, and binding tracks 06-11.

Bootstrap fixtures are flat JSON files in `conformance/fixtures/` with a manifest in `conformance/fixtures/manifest.json`. They are intentionally small and language-neutral so each track can consume the same source of truth without reinterpreting the semantics.

The directory-backed fixture shape in `conductor/contracts/conformance-contract.md` remains the target runner contract once the shared runner lands. The bootstrap files in this track are the concrete inputs downstream tracks can use before that runner exists.

Required fixture families for the first wave:

```text
scheduler_ordering_v1
scheduler_cancellation_v1
zero_delay_guard_v1
rng_reproducibility_v1
des_resource_queue_v1
abm_behavior_update_v1
hybrid_des_abm_v1
arrow_event_log_v1
ffi_lifecycle_v1
```

Current bootstrap files with ready status:

```text
conformance/fixtures/deterministic_ordering.json
conformance/fixtures/cancellation.json
conformance/fixtures/rng_replay.json
conformance/fixtures/vvuq_scenario_replay.json
conformance/fixtures/zero_delay_guard.json
```

Each ready fixture must define:

- `fixture`
- `version`
- deterministic input state
- an expected ordering, summary, or reproducibility assertion
- a stable consumer contract in `conformance/fixtures/manifest.json`

Downstream consumer expectations:

- Track 01 uses the ordering, cancellation, zero-delay, and RNG fixtures as the source of truth for core scheduler/state behavior.
- Track 02 uses the FFI lifecycle fixture to prove that the facade can round-trip the same core behavior without ownership leaks.
- Tracks 06-11 use the shared manifest and fixture IDs directly; they should not rename fixture semantics locally.

## Benchmark strategy

Track 12 owns the benchmark plan and keeps the benchmark scenario names stable for Track 01 and the binding tracks.

The first benchmark scenarios are:

```text
schedule_1m_events
pop_1m_events
schedule_cancel_1m_mixed
create_1m_entities
component_insert_1m
hybrid_des_abm_smoke_100k
```

Benchmark expectations:

- Track 01 owns the core Rust kernels and baseline numbers.
- Track 02 measures facade overhead against the same kernels.
- Tracks 06-11 measure binding overhead against the same scenario names and report the wrapper cost separately from the core kernel cost.
- Track 18 consumes the published baseline, not a new naming scheme.

### Chaos engineering

The conformance suite MUST include chaos experiments that verify the system degrades gracefully:

Fault injection types:
- **Event corruption**: inject malformed event data (negative ticks, null entity, invalid priority).
- **Entity exhaustion**: spawn entities until handle space is exhausted.
- **Telemetry loss**: truncate Arrow IPC output mid-write.
- **Ordering inversion**: feed events in reverse-time order.

Resilience expectations:
- No panic, no abort, no UB (undefined behaviour) for any injected fault.
- Corrupted events produce `KAIRO_ECS_ERR_INVALID_ARGUMENT`, not silent misbehavior.
- Exhausted entity space produces a clear error, not a crash or wraparound.
- Telemetry truncation produces a valid partial Arrow IPC file with an error marker.

### Deep fuzzing

Beyond the single `cargo-fuzz` target, structure-aware fuzzing MUST cover:

- `#[derive(Arbitrary)]` on all public types (SimTime, SimDuration, EventId, EntityId, ScheduleRequest, DispatchedEvent, StepOutcome).
- Structure-aware fuzz target: generate random event sequences, feed to scheduler, verify ordering invariants hold.
- Differential fuzzing: run identical event sequences through Rust C ABI and Python/R/Julia/TS/C#/Go bindings, verify identical output.
- OSS-Fuzz integration: register kairo-ecs-core and kairo-ecs-ffi fuzz targets with Google OSS-Fuzz for continuous fuzzing.

Chaos and fuzzing checks are release-gating for beta and beyond.

The current checked-in chaos slice is metadata-only:

```text
conformance/chaos/manifest.json
```

It records the required fault-family contract without native link tests,
nightly scheduling, a checked-in chaos runner, or runtime fault injection claims.

## Acceptance criteria

- Owned paths are created and documented.
- Contract inputs and outputs are explicit.
- Track tests or validation checks exist.
- CI gate is defined.
- Documentation impact is recorded.
- Release implications are recorded.
- `handoff.md` is completed before merge.

## Quality gates

Use the gates in `conductor/quality-gates.md`. Track-specific gates must be listed in `test-matrix.md`.

## Release implications

This track contributes to release readiness only through the acceptance criteria and quality gates listed here and in conductor/quality-gates.md. It does not independently authorize public release, registry publication, or production-readiness claims without the dependent packaging, supply-chain, compatibility, red-team, and wave-management gates.
