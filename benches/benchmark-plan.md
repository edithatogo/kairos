# Benchmark Plan

## Scope

This plan defines the canonical benchmark scenario registry for Track 12.
Track 01 owns the core kernels, Track 02 measures facade overhead, and Tracks
06-11 measure binding overhead against the same scenario names.

The metadata-only smoke harness in `benches/benchmark-smoke.json` and
`benches/benchmark_smoke.py` must stay aligned with this plan.

## Canonical scenarios

| Scenario ID | Owner | Smoke scale | Scale | What it measures | Binding consumer behavior |
|---|---|---:|---:|---|---|
| `schedule_1m_events` | `01` | `4` | `1_000_000` | scheduler insertion throughput | Measure wrapper overhead against the same kernel |
| `pop_1m_events` | `01` | `4` | `1_000_000` | queue drain throughput | Measure wrapper overhead against the same kernel |
| `schedule_cancel_1m_mixed` | `01` | `4` | `1_000_000` | cancellation stability under load | Reuse the same scenario name and input mix |
| `create_1m_entities` | `01` | `4` | `1_000_000` | entity allocator throughput | Reuse the same scenario name and report wrapper cost |
| `component_insert_1m` | `01` | `4` | `1_000_000` | component store throughput | Reuse the same scenario name and report wrapper cost |
| `hybrid_des_abm_smoke_100k` | `01` | `4` | `100_000` | mixed DES and ABM smoke path | Reuse the same scenario name and report wrapper cost |

## Measurement contract

- Benchmark names must stay stable once published.
- Core kernel metrics come from Track 01.
- Track 02 measures the stable facade on top of the same kernels.
- Tracks 06-11 measure binding overhead against the same names and do not invent local naming schemes.
- Track 18 consumes the published baseline and metadata, not a renamed benchmark family.
- The smoke manifest stays metadata-only: `harness = metadata-only` and
  `requires_native_link_tests = false`.
- Smoke scales remain reduced CI scales only and must match
  `benches/benchmark-smoke.json`.

## Recommended outputs

- Wall-clock time
- Throughput or ops/s
- Allocation count or peak memory where the benchmark harness can observe it
- Baseline comparison against the previous accepted run

## Smoke metadata

`benches/benchmark-smoke.json` records a metadata-only smoke harness for the
canonical scenarios. It intentionally sets `requires_native_link_tests` to
`false` so Track 12 can validate scenario names, ownership, and smoke scales
before the native benchmark binaries are integrated.

## Acceptance criteria

- All canonical scenario names appear in `conductor/tracks/12-conformance-testing-benchmarks/spec.md`.
- The benchmark plan is present in the repo and referenced by `test-matrix.md`.
- `python benches/benchmark_smoke.py` validates metadata against `conformance/fixtures/manifest.json`.
- No binding track redefines the scenario names without a manifest update.
