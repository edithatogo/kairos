# 01 The Heart: kairo-ecs-core & kairo-ecs-state — plan.md

## Phase 0 — Track startup

- Read `conductor/workflow.md`.
- Read `conductor/product.md`, `conductor/tech-stack.md`, `conductor/workflow.md`, and the scheduler/state/type contracts under `conductor/contracts/`.
- Confirm owned paths: `crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, `crates/kairo-ecs-rng`, `conformance/fixtures`, `benches`, and `crates/kairo-ecs-ffi` only for the FFI-readiness boundary.
- Confirm the current control artifacts that constrain this track: `lanes.md`, `conformance/fixtures/manifest.json`, `scripts/validate_conductor_setup.ps1`, and `scripts/validate_track_coverage.ps1`.
- Refresh `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md` to match the active repo state.
- Use `lanes.md` to split work into types/time, scheduler, state, RNG, and facade readiness.

## Phase 1 — Contract alignment

- Define `SimTime`, `SimDuration`, event ordering, entity IDs, scheduler semantics, and run-seed behavior through `crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, and `crates/kairo-ecs-rng`.
- Propose contract changes through ADR if required.
- Keep the core track aligned with the deterministic fixture manifest, `lanes.md`, and the bootstrap conformance fixtures.

## Phase 2 — Scaffold

- Extend the existing crate skeleton with the concrete scheduler, state, and RNG surfaces that already have workspace roots.
- Add smoke tests that prove the core workspace and fixture manifest are wired into CI.
- Keep the lane docs aligned with the concrete contracts and fixtures that already exist.

## Phase 3 — Implementation

- Implement the smallest useful vertical slice for deterministic ordering, cancellation, and reproducible seeds.
- Add unit tests and integration tests.
- Add Track 12 fixture parity checks where the core scheduler and state surfaces consume them.
- Add the scheduler and ECS benchmarks that the core track owns.

## Phase 4 — Cross-track integration

- Run owned tests.
- Run affected shared conformance tests.
- Update docs and release notes.
- Ensure no other subagent-owned paths were modified without handoff.

## Phase 5 — Closeout

- Complete `handoff.md`.
- Record risks and follow-up tasks.
- Confirm CI gates.
- Mark track ready for integration.


## Detailed phases

### Phase 1 — Contracts

- Define `SimTime`, `SimDuration`, IDs, errors, event kind.
- Write deterministic ordering fixture.
- Write ADR for time representation.
- Write ADR for handle generation strategy.
- Keep `crates/kairo-ecs-types` free of host-language dependencies.

### Phase 2 — Priority queue

- Implement stable heap ordering.
- Add insertion sequence.
- Add cancellation marker or generational event table.
- Property-test ordering and cancellation.
- Keep the deterministic ordering fixture aligned with `conformance/fixtures/deterministic_ordering.json`.

### Phase 3 — Run loop

- Implement `step`, `run_for`, `run_until`, `run_until_or_for`.
- Add zero-delay guardrail.
- Add tracing spans.
- Add stats collection.

### Phase 4 — ECS storage

- Implement entity allocator.
- Implement component insertion/removal.
- Implement query API needed by DES/ABM.
- Benchmark 1M entities.

### Phase 5 — RNG

- Implement run seed.
- Implement entity stream derivation.
- Add reproducibility fixture.

### Phase 6 — FFI readiness

- Create a pure Rust facade that maps cleanly to handles/status codes.
- Freeze minimal API for Track 02.
- Add docs for binding agents.
- Do not let binding tracks start from Track 01 alone; require Track 02 and Track 12 readiness as defined in `lanes.md`.

## Phase 6 — SIMD acceleration

- Gate: ECS storage strategy ADR is accepted and component storage is implemented.
- Implement `std::simd` batch operations on the chosen storage layout (sparse-set, slotmap, or archetype).
- Add SIMD-vs-scalar benchmark scenarios to the Track 12 benchmark harness.
- Document autovectorisation guidance in the core performance docs.
- Update `conductor/performance-thresholds.md` with SIMD baseline and regression thresholds.

## Phase 7 — Formal verification

- Gate: Sequential scheduler determinism is proven by Track 12 conformance fixtures passing on all platforms.
- Write Kani proofs for scheduler ordering invariants.
- If threading is added (Track 34), write loom tests for concurrent queue safety.
- Add Creusot contracts to scheduler public API.
- Publish verification results in `docs/verification/` and link from release notes.

