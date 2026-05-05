# 01 The Heart: kairo-ecs-core & kairo-ecs-state — spec.md

## Mission

Build the deterministic Rust heart: SimTime, event scheduler, priority queue, cancellation, bounded run loop, entity/component storage, and 1M+ entity performance foundation.

## Primary subagent

```text
core-scheduler-agent + ecs-agent + contracts-agent
```

## Dependencies

```text
Track 00 skeleton. Contract phase can begin immediately after repo setup.
```

## Owned paths

```text
crates/kairo-ecs-types, crates/kairo-ecs-core, crates/kairo-ecs-state, crates/kairo-ecs-rng
```

## Blocked paths

```text
crates/kairo-ecs-ffi/ — owned by Track 02 (FFI bridge)
crates/kairo-ecs-des/ — owned by Track 03 (DES API)
crates/kairo-ecs-abm/ — owned by Track 03 (ABM API)
include/ — owned by Track 02 (C headers)
.github/ — owned by Track 13 (CI/CD)
bindings/ — owned by Tracks 06-11
```

Track 01 is executed through the lanes in `conductor/tracks/01-heart-kairo-ecs-core-state/lanes.md`.

## Release implications

- Any change to event ordering semantics or SimTime representation requires an ADR and is breaking.
- Adding new public API surfaces is additive and safe within the same major version.
- Performance regression in any of the 6 benchmark scenarios blocks release.
- Unsafe code in core crates requires an ADR.

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


## Detailed Track 01 specification

### Scope

Track 01 is the high-performance Rust heart of KairoECS:

```text
kairo-ecs-types: SimTime, SimDuration, EventId, EntityId, errors, versioned DTOs
kairo-ecs-core: scheduler, priority queue, cancellation, run loop, event dispatch stats
kairo-ecs-state: entity/component storage, archetype or sparse-set layout decision, entity lifecycle
kairo-ecs-rng: deterministic per-run/per-entity random streams
```

### Hard requirements

1. Support 1,000,000+ entity handles without pathological memory overhead.
2. Support nanosecond precision through fixed tick time, not raw floating-point ordering.
3. Dispatch events deterministically by `(time, priority, sequence)`.
4. Provide cancellation without breaking heap invariants.
5. Provide bounded run loops to avoid accidental infinite zero-delay loops.
6. Keep host-language objects out of the hot path.
7. Keep `unsafe` out of core crates unless an ADR approves it.
8. Expose enough pure Rust facade functionality for Track 02 FFI and Track 12 conformance tests.

### Current skeleton

The first executable slice now exists:

```text
crates/kairo-ecs-types
crates/kairo-ecs-core
crates/kairo-ecs-state
crates/kairo-ecs-rng
```

This skeleton is intentionally small. It proves workspace wiring, deterministic event ordering, cancellation, entity lifecycle, and reproducible RNG streams before deeper ECS and FFI design work starts.

### Scheduler API shape

```rust
pub struct SimTime { ticks: u128 }
pub struct SimDuration { ticks: u128 }
pub struct EventId { /* generational handle */ }
pub struct EntityId { /* generational handle */ }

pub struct ScheduleRequest {
    pub at: SimTime,
    pub priority: i32,
    pub entity: Option<EntityId>,
    pub kind: EventKind,
}

pub enum StepOutcome {
    Dispatched(DispatchedEvent),
    Empty,
    LimitReached,
}
```

### ECS design criteria

Decide via ADR whether v1 uses:

```text
custom sparse-set ECS
slotmap-backed component stores
archetype ECS
standalone hecs-like approach
```

The default recommendation is a custom or minimal sparse-set/slotmap design because FFI stability and deterministic serialization matter more than game-engine ergonomics.

### Performance benchmarks

Minimum benchmark scenarios:

```text
schedule_1m_events
pop_1m_events
schedule_cancel_1m_mixed
create_1m_entities
component_insert_1m
hybrid_des_abm_smoke_100k
```



### Phase 6 — SIMD acceleration

Once the ECS storage strategy is settled via ADR 0001, SIMD vectorisation MUST be applied to component batch operations:

- Use `std::simd` (Rust nightly, stabilised for portable SIMD) for batch iteration over component columns.
- Target operations: component insertion in batches, system execution over matching archetypes, Arrow column encoding/decoding in Track 04.
- Autovectorisation guidance: prefer `ChunksExact` iterators over scalar loops; use `#[rustc_auto_vectorize]` or equivalent pragmas where possible.
- Benchmark thresholds: SIMD path must show ≥2x throughput improvement on component_insert_1m vs scalar baseline.

### Phase 7 — Formal verification

After the sequential scheduler is proven deterministic via Track 12 conformance fixtures, apply formal methods to key scheduler invariants:

- **Kani** (`kani` crate): prove that `(time, priority, sequence)` ordering is total, transitive, and deterministic — no two runs with the same seed can produce different event dispatch orders.
- **loom** (`loom` crate): if threading is added (Track 34 PDES), verify that the event queue is free of data races under concurrent push/pop.
- **Creusot** (`creusot` crate): specify and verify that `cancel()` does not break heap invariants (no event can be dispatched after its cancel returns true).

Formal verification is optional and does not block release. It is a continuous improvement target.


## Acceptance criteria

- Owned paths are created and documented.
- Track lanes are updated when ownership or scope changes.
- Contract inputs and outputs are explicit.
- Track tests or validation checks exist.
- CI gate is defined.
- Documentation impact is recorded.
- Release implications are recorded.
- `handoff.md` is completed before merge.


## Quality gates

Use the gates in `conductor/quality-gates.md`. Track-specific gates must be listed in `test-matrix.md`.



