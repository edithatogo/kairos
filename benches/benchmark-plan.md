# Benchmark Plan

## Scope

This plan defines the benchmark scenario names and measurement contract for Track 01, Track 02, and Tracks 06-11.

## Canonical scenarios

| Scenario | What it measures | Primary owner | Binding consumer behavior |
|---|---|---|---|
| `schedule_1m_events` | Scheduler insertion throughput | Track 01 | Measure wrapper overhead against the same kernel |
| `pop_1m_events` | Queue drain throughput | Track 01 | Measure wrapper overhead against the same kernel |
| `schedule_cancel_1m_mixed` | Cancellation stability under load | Track 01 | Reuse the same scenario name and input mix |
| `create_1m_entities` | Entity allocator throughput | Track 01 | Reuse the same scenario name and report wrapper cost |
| `component_insert_1m` | Component store throughput | Track 01 | Reuse the same scenario name and report wrapper cost |
| `hybrid_des_abm_smoke_100k` | Mixed DES and ABM smoke path | Track 01 | Reuse the same scenario name and report wrapper cost |

## Measurement contract

- Benchmark names must stay stable once published.
- Core kernel metrics come from Track 01.
- Track 02 measures the stable facade on top of the same kernels.
- Tracks 06-11 measure binding overhead against the same names and do not invent local naming schemes.
- Track 18 consumes the published baseline and metadata, not a renamed benchmark family.

## Recommended outputs

- Wall-clock time
- Throughput or ops/s
- Allocation count or peak memory where the benchmark harness can observe it
- Baseline comparison against the previous accepted run

## Acceptance criteria

- All canonical scenario names appear in `conductor/tracks/12-conformance-testing-benchmarks/spec.md`.
- The benchmark plan is present in the repo and referenced by `test-matrix.md`.
- No binding track redefines the scenario names without a manifest update.
