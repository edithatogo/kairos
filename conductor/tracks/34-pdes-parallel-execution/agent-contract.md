# Agent Contract: pdes-agent

## Track

Track 34: PDES & Parallel Execution

## Owned paths

- `conductor/tracks/34-pdes-parallel-execution/`
- `crates/kairo-ecs-pdes/`
- `docs/pdes/`
- `benches/pdes/`
- Track-specific artifacts named in `plan.md`

## Required handoff

- Summary of PDES architecture (CMB algorithm, LP model, GVT algorithm).
- `LogicalProcess` trait specification and usage examples.
- Event exchange protocol specification with lookahead semantics.
- Benchmark results for 4/8/16/32 LP configurations versus sequential baseline.
- Time Warp research spike findings and recommendation.
- Integration guide for Track 35 (distributed simulation) — how to replace the communication layer.
- Follow-up items for Track 01 (core scheduler) and Track 12 (benchmarks).

## Prohibited changes without ADR

- Modifying `crates/kairo-ecs-core/` scheduler internals — the sequential scheduler path must remain untouched.
- Modifying `docs/core-contract.md` — the determinism contract is owned by Track 01.
- Changing the `LogicalProcess` trait after it stabilizes without a migration path.
- Removing or weakening the `pdes` feature flag gating.
- Introducing PDES code paths that execute when the `pdes` feature is disabled.

## Gate contract

### pdes-sequential-parity
- **Input**: Sequential scheduler benchmark output, PDES benchmark output for a partitioned world.
- **Output**: Pass if PDES final state (component data, entity graph) is identical to sequential final state for the same initial conditions. Fail with the specific component/entity that diverges.
- **Blocking**: Yes for PDES feature flag — blocks enabling `pdes` by default. Not release-gating for sequential mode.

### gvt-progression-check
- **Input**: PDES run with GVT logging enabled, minimum tick count.
- **Output**: Pass if GVT increases monotonically and advances at least every N ticks (where N is configurable per LP count). Fail with the tick range where GVT stalled.
- **Blocking**: Yes for PDES feature flag. Not release-gating for sequential mode.

### pdes-deadlock-free
- **Input**: Stress test run (10,000+ ticks, random events across 8 LPs).
- **Output**: Pass if the simulation completes without deadlock (terminates within timeout). Fail on timeout or stalled progress.
- **Blocking**: Yes for PDES feature flag. Not release-gating for sequential mode.
