# Track 34: PDES & Parallel Execution

## Purpose

Add parallel discrete event simulation (PDES) to the KairoECS scheduler. Conservative PDES (Chandy-Misra-Bryant algorithm) first — each logical process (LP) maintains its own event queue and exchanges events with lookahead. Optimistic PDES (Time Warp) as a research spike for a later iteration.

## Why this track exists

The sequential scheduler from Track 01 is proven deterministic but cannot exploit multi-core machines. PDES enables partitioning a simulation world across logical processes running on separate threads or cores, trading deterministic ordering for throughput. This track builds the conservative PDES path so that partitioned worlds benefit from parallelism while producing identical final states to the sequential scheduler.

## Primary subagent

`pdes-agent`

## Parallelization model

Depends on Track 01 (sequential scheduler — must be proven deterministic first) and Track 12 (benchmark harness). The sequential scheduler's deterministic ordering contract is the correctness oracle. PDES work runs in its own crate and does not modify core scheduler internals. Cross-track integration with Track 04 (Arrow telemetry) is deferred until PDES is stable.

## Inputs

- `crates/kairo-ecs-core/` — sequential scheduler from Track 01 and deterministic ordering contract from `docs/core-contract.md`.
- `benches/` — benchmark harness from Track 12.
- `docs/core-contract.md` — deterministic ordering specification from Track 01.

## Outputs

- `crates/kairo-ecs-pdes/` — new crate containing:
  - Conservative PDES scheduler implementation (CMB algorithm).
  - `LogicalProcess` trait defining LP lifecycle (init, process-local-events, exchange-events, advance-GVT).
  - GVT (global virtual time) calculation algorithm.
  - Event exchange protocol between LPs with lookahead guarantees.
- `docs/pdes/` — design documentation covering LP partitioning strategy, lookahead computation, GVT algorithm, and known limitations.
- PDES benchmark suite (`benches/pdes/`) measuring scaling for 4/8/16/32 LP configurations against sequential baseline.

## Owned paths

- `crates/kairo-ecs-pdes/`
- `docs/pdes/`
- `benches/pdes/`
- `conductor/tracks/34-pdes-parallel-execution/`

## Blocked paths

- `crates/kairo-ecs-core/` — owned by Track 01 (scheduler internals).
- `benches/` (non-pdes) — owned by Track 12.
- `docs/core-contract.md` — owned by Track 01.
- `crates/kairo-ecs-arrow/` — owned by Track 04.

## Acceptance criteria

- Conservative PDES produces identical final state to sequential scheduler for partitioned worlds (sequential parity test in test-matrix).
- Speedup of 2x+ on 4 cores, 4x+ on 8 cores versus sequential baseline for partitioned workloads.
- GVT progresses monotonically without deadlock under representative simulation loads.
- Deadlock-free stress test passes for 10,000+ tick simulations with random event patterns across 8 LPs.
- `LogicalProcess` trait is documented with usage examples in `docs/pdes/`.
- Gated behind cargo feature flag `pdes`. Not enabled by default.

## Release implications

PDES is a performance enhancement — does not affect API compatibility for sequential users. Gated behind cargo feature flag `pdes`. Does not block release if sequential scheduler is correct. Enabling PDES for a given simulation requires explicit world partitioning by the user.

## Non-goals

- Replacing the sequential scheduler for non-partitioned worlds.
- Guaranteeing deterministic PDES ordering (by definition non-deterministic across LPs — final state parity is the goal, not per-tick ordering parity).
- Optimistic PDES (Time Warp) implementation — research spike only in this track.
- Automatic LP partitioning or workload balancing.
- Integration with distributed simulation (Track 35) — that track consumes the LP model from this one.
