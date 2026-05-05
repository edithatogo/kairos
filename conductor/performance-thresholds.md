# Performance Thresholds

This document records the initial regression thresholds for the visible benchmark set.

## Benchmarks

| Benchmark | Baseline | Threshold | Notes |
|---|---:|---:|---|
| `schedule_1m_events_preview` | 1.0x baseline | 10% regression | Track 12 benchmark preview |
| `hybrid_des_abm_smoke_preview` | 1.0x baseline | 10% regression | Track 12 hybrid smoke preview |

## Control rule

If a benchmark regresses beyond threshold, the PR must fail until the regression is understood and either fixed or explicitly accepted.
