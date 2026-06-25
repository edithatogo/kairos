# Track 47: PDES Conservative Lookahead Production Runtime

## Purpose

Promote the Track 34 PDES scaffold into a production conservative PDES runtime
with enforced lookahead, LP partitioning, GVT progression, deadlock prevention,
sequential parity, and measured scaling evidence.

## Maturity

Spec Approved planning track. The current implementation remains the Track 34
scaffold until this track's TDD implementation phases close.

## Inputs

- `crates/kairo-ecs-pdes/` and `docs/pdes/` from Track 34.
- Sequential scheduler contracts from Track 01.
- Benchmark and reproducibility policy from Tracks 12, 18, and 31.
- HPC evidence contract from Track 46.

## Outputs

- Real conservative PDES scheduler behind the `pdes` feature.
- LP partitioning API with explicit lookahead and safe-time contracts.
- Deadlock stress tests and sequential final-state parity tests.
- 4/8/16/32 LP benchmark profiles with raw evidence manifests.

## Owned paths

- `crates/kairo-ecs-pdes/`
- `benches/pdes/`
- `docs/pdes/`
- `conductor/tracks/47-pdes-conservative-lookahead-production-runtime/`

## Blocked paths

- `crates/kairo-ecs-core/` scheduler internals without Track 01 handoff.
- Real distributed transports owned by Track 49.
- Scaling certification rollup owned by Track 55.

## Dependencies

Tracks 34 and 46.

## Parallel-safe tracks

Tracks 48, 49, and 55 may draft tests against the public LP contract but must
not change the conservative runtime semantics without this track's handoff.

## Acceptance criteria

- Conservative PDES produces final-state parity with the sequential scheduler
  across deterministic DES, ABM, and mixed workloads.
- Lookahead violations fail fast with typed errors.
- GVT progresses monotonically under random and adversarial event patterns.
- Deadlock stress tests cover at least 10,000 simulated ticks and 8 LPs.
- Benchmarks record raw throughput, LP count, core count, topology, and seeds.

## Quality gates

- `pdes-production-lookahead`
- `pdes-sequential-parity`
- `gvt-progression-check`
- `pdes-deadlock-free`
- `hpc-evidence-manifest`
- `phase-closeout-check`

## Release implications

This track is release-critical for any production PDES claim. Sequential users
must remain unaffected unless the `pdes` feature is enabled.
