# Handoff: Track 31 Performance Regression Guard

## Summary

Defined the performance regression detection framework for KairoECS. Established per-benchmark thresholds covering scheduler throughput, entity operations, component iteration, event dispatch, and serialization. Built comparison scripts and a CI workflow (`bench-regression.yml`) that runs on PRs touching core crates or benchmarks and fails when a regression exceeds the defined threshold.

## Files changed

`conductor/tracks/31-performance-regression-guard/plan.md`, `conductor/tracks/31-performance-regression-guard/spec.md`, `conductor/tracks/31-performance-regression-guard/test-matrix.md`, `conductor/tracks/31-performance-regression-guard/handoff.md`, `conductor/performance-thresholds.md`, `benches/regression/compare.py`, `.github/workflows/bench-regression.yml`, `conductor/quality-gates.md`

## Contracts consumed

- `benches/` and `crates/kairo-ecs-bench/` — benchmark harnesses from Track 12 (read-only).
- `docs/benchmarks/` — comparative benchmark metadata from Track 18 (read-only).
- Existing CI workflow patterns from Track 13 (read-only reference).

## Release gates affected

- **benchmark-regression-check**: Blocks PR merge if any benchmark exceeds its threshold. Not release-gating.
- **threshold-definition-exists**: Blocks PR merge if any active benchmark lacks a threshold entry. Not release-gating.
- Both gates are in `conductor/quality-gates.md`. This track is explicitly non-critical for release — a documented and accepted regression does not block release.

## Risks and unresolved questions

- CI runner heterogeneity is the primary risk to baseline stability. Controlled-runner pinning may not be feasible on GitHub Actions free tier — consider self-hosted runners or nightly full-suite runs with PR-only sampling.
- Threshold tuning is an ongoing activity. Initial values (3-10% depending on category) are estimates and should be reviewed quarterly against accumulated benchmark history.
- Benchmarks added by Track 12 or Track 18 after this track ships must include threshold entries; the `threshold-definition-exists` gate enforces this at the PR level.
