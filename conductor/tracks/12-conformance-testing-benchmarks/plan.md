# 12 Conformance, Testing & Benchmarks — plan.md

## Phase 0 — Track startup

- Read `conductor/workflow.md`.
- Read relevant contracts under `conductor/contracts/`.
- Confirm owned paths: `conformance, tests/conformance, benches, crates/kairo-ecs-bench`.
- Keep the shared fixture IDs and benchmark scenario names stable once published.
- Create `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md`.

## Phase 1 — Contract alignment

- Identify all public types, functions, schemas, commands, or package metadata this track consumes.
- Propose contract changes through ADR if required.
- Freeze the first fixture manifest entries for `scheduler_ordering_v1`, `scheduler_cancellation_v1`, and `rng_reproducibility_v1`.
- Keep fixture JSON small, versioned, and language-neutral.
- Document which downstream tracks consume each fixture family.

## Phase 2 — Scaffold

- Create package/crate/module skeleton.
- Add fixture-runner smoke tests that prove the shared manifest is wired into CI.
- Add docs stubs that point open follow-ups to Linear or Conductor tasks.
- Add a shared runner contract in `tests/conformance/README.md`.
- Publish a benchmark plan in `benches/benchmark-plan.md`.

## Phase 3 — Implementation

- Implement the smallest useful vertical slice.
- Add unit tests and integration tests.
- Add fixture parity tests for every track that consumes the shared manifest.
- Add scheduler, queue, and entity benchmarks that the conformance track owns.
- Keep benchmark names aligned with `conductor/tracks/01-heart-kairo-ecs-core-state/spec.md`.

## Phase 4 — Cross-track integration

- Run owned tests.
- Run affected shared conformance tests.
- Update docs and release notes.
- Ensure no other subagent-owned paths were modified without handoff.
- Verify Track 01, Track 02, and Track 06-11 can consume the manifest without renaming fixtures.

## Phase 5 — Closeout

- Complete `handoff.md`.
- Record risks and follow-up tasks.
- Confirm CI gates.
- Mark track ready for integration.

## First fixtures

- Deterministic ordering by `(time, priority, sequence)`.
- Cancellation without reordering remaining events.
- RNG replay from a run seed and entity handle.
- Zero-delay guardrail fixture for the run loop.
- Arrow event-log fixture once Track 04 defines the schema.

## Benchmark plan

- `schedule_1m_events`: scheduler insertion throughput.
- `pop_1m_events`: queue drain throughput.
- `schedule_cancel_1m_mixed`: cancellation stability under load.
- `create_1m_entities`: entity allocator throughput.
- `component_insert_1m`: component store throughput.
- `hybrid_des_abm_smoke_100k`: mixed scheduler/ECS smoke benchmark.

