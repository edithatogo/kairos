# Agent Contract: perf-regression-agent

## Track

Track 31: Performance Regression Guard

## Owned paths

- `conductor/tracks/31-performance-regression-guard/`
- `conductor/performance-thresholds.md`
- `benches/regression/`
- `.github/workflows/bench-regression.yml`
- Track-specific artifacts named in `plan.md`

## Required handoff

- Summary of all benchmark thresholds and their pass/fail criteria.
- Comparison script output format and CI integration details.
- Gate definitions and their pass/fail semantics.
- List of benchmarks that could not produce a stable baseline.
- Follow-up items for benchmark (Track 12) and CI (Track 13) subagents.

## Prohibited changes without ADR

- Modifying benchmark harness code in `benches/` or `crates/kairo-ecs-bench/` (owned by Track 12).
- Modifying comparative benchmark scenarios (owned by Track 18).
- Changing the baseline measurement methodology without updating `conductor/performance-thresholds.md`.
- Lowering a threshold to allow a known regression without documented rationale.
- Modifying CI workflows other than `bench-regression.yml`.

## Gate contract

### benchmark-regression-check
- **Input**: PR branch benchmark output, base branch benchmark output, `conductor/performance-thresholds.md`.
- **Output**: Pass if all benchmarks are within their defined thresholds. Fail with the benchmark name, base value, PR value, percentage change, and threshold.
- **Blocking**: Yes for PR merge — blocks the PR if regression exceeds threshold. Not release-gating.

### threshold-definition-exists
- **Input**: `conductor/performance-thresholds.md`, benchmark inventory from `benches/` and `crates/kairo-ecs-bench/`.
- **Output**: Pass if every active benchmark has a threshold entry. Fail with the list of benchmarks missing thresholds.
- **Blocking**: Yes for PR merge — blocks PRs that add benchmarks without thresholds.
