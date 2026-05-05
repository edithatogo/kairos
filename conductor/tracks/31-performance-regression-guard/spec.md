# Track 31: Performance Regression Guard

## Purpose

Own automated performance regression detection — define thresholds, add CI comparison gates, and prevent benchmarks from silently degrading.

## Why this track exists

KairoECS markets itself as a high-performance ECS kernel. Benchmarks exist (Track 12, Track 18) but without automated regression detection, a PR can degrade performance by 10% or more and pass all correctness gates. This track closes that gap by defining thresholds, wiring comparison checks into CI, and blocking regressions.

## Primary subagent

`perf-regression-agent`

## Parallelization model

This track depends on the benchmark harness (Track 12) and comparative benchmarks (Track 18) being at least scaffolded. It starts once those tracks produce runnable benchmarks. It does not modify core ECS code or add new benchmark scenarios.

## Inputs

- `benches/` — benchmark harnesses from Track 12.
- `docs/benchmarks/` — comparative benchmark metadata from Track 18.
- `crates/kairo-ecs-bench/` — benchmark crate from Track 12.
- Existing CI workflow patterns from Track 13 (read-only reference).

## Outputs

- A threshold definition document (`conductor/performance-thresholds.md`) naming every benchmark, its baseline value, the acceptable regression percentage, and the measurement methodology.
- CI regression workflow (`.github/workflows/bench-regression.yml`) that runs benchmarks, compares against the baseline, and fails on regression exceeding threshold.
- Test data and comparison scripts in `benches/regression/`.
- Gate definitions for `benchmark-regression-check` and `threshold-definition-exists`.
- Handoff notes for CI, benchmark, and release subagents.

## Owned paths

- `conductor/performance-thresholds.md`
- `benches/regression/`
- `.github/workflows/bench-regression.yml`
- `conductor/tracks/31-performance-regression-guard/`

## Blocked paths

- Core benchmark harness code in `benches/` and `crates/kairo-ecs-bench/` — owned by Track 12.
- Comparative benchmark scenarios in `docs/benchmarks/` — owned by Track 18.
- CI/CD workflow infrastructure — owned by Track 13.
- Release packaging — owned by Track 15.

## Acceptance criteria

- Every benchmark in `benches/` and `crates/kairo-ecs-bench/` has a named threshold in `conductor/performance-thresholds.md`.
- `bench-regression.yml` runs on PRs that modify `crates/kairo-ecs-core/`, `crates/kairo-ecs-state/`, or `benches/`.
- A regression exceeding the threshold produces a CI failure with the benchmark name, baseline value, current value, and percentage change.
- The `threshold-definition-exists` gate fails if any active benchmark lacks a threshold entry.
- The `benchmark-regression-check` gate fails if any benchmark exceeds its threshold on the PR branch versus the base branch.

## Release implications

- This track is **non-critical** for release. It improves quality but does not gate release.
- A performance regression that exceeds the threshold blocks the PR merge but does not block the release if the regression is documented and accepted.
- Thresholds are advisory for release planning but become blocking for PRs after Track 31 reaches beta.

## Non-goals

- Writing new benchmarks (owned by Track 12 and Track 18).
- Optimizing code for performance (owned by Track 01).
- Defining comparative benchmark baselines against ecosystem projects (owned by Track 18).
- Changing the benchmark harness API (owned by Track 12).
