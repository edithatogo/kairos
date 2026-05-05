# Performance Thresholds

This document records the initial regression thresholds for the canonical benchmark
set declared by `benches/benchmark-smoke.json` and
`conformance/fixtures/manifest.json`.

Threshold rows intentionally use the canonical scenario IDs, not local preview
aliases. Baseline values remain relative until Track 12 publishes native
Criterion outputs; the local regression validator still checks that every
canonical benchmark has exactly one threshold row.

## Benchmarks

| Benchmark | Measure | Owner | Baseline | Regression threshold | Method | Gate | Metadata source |
|---|---|---:|---:|---:|---|---|---|
| `schedule_1m_events` | scheduler insertion throughput | 01 | 1.0x accepted baseline | 5% | relative mean wall-clock delta; lower is better | blocking | `benches/benchmark-smoke.json`; `conformance/fixtures/manifest.json` |
| `pop_1m_events` | queue drain throughput | 01 | 1.0x accepted baseline | 5% | relative mean wall-clock delta; lower is better | blocking | `benches/benchmark-smoke.json`; `conformance/fixtures/manifest.json` |
| `schedule_cancel_1m_mixed` | cancellation stability under load | 01 | 1.0x accepted baseline | 5% | relative mean wall-clock delta; lower is better | blocking | `benches/benchmark-smoke.json`; `conformance/fixtures/manifest.json` |
| `create_1m_entities` | entity allocator throughput | 01 | 1.0x accepted baseline | 3% | relative mean wall-clock delta; lower is better | blocking | `benches/benchmark-smoke.json`; `conformance/fixtures/manifest.json` |
| `component_insert_1m` | component store throughput | 01 | 1.0x accepted baseline | 3% | relative mean wall-clock delta; lower is better | blocking | `benches/benchmark-smoke.json`; `conformance/fixtures/manifest.json` |
| `hybrid_des_abm_smoke_100k` | mixed DES and ABM smoke path | 01 | 1.0x accepted baseline | 10% | relative mean wall-clock delta; lower is better; advisory until native hybrid benchmark is promoted | advisory | `benches/benchmark-smoke.json`; `conformance/fixtures/manifest.json` |

## Control rule

If a benchmark regresses beyond threshold, the PR must fail until the regression is understood and either fixed or explicitly accepted.

## Accepted result JSON

`benches/regression/compare.py` accepts either a list of benchmark result objects
or an object with a `benchmarks` array. Each result must include an ID and a mean
duration using one of these common keys:

- ID: `id`, `name`, `benchmark`, or `scenario`
- Mean duration: `mean`, `mean_seconds`, `mean_ms`, `time`, or
  Criterion-style `estimates.mean.point_estimate`

Durations are compared as lower-is-better wall-clock means. Throughput metrics
must be converted to durations before being passed to the guard.
