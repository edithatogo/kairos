# Test Matrix: Track 31 Performance Regression Guard

| Check | Alpha | Beta | RC | 1.0 |
|---:|---:|---:|---:|---:|
| Track docs exist and render cleanly | yes | yes | yes | yes |
| `conductor/performance-thresholds.md` exists and lists every active benchmark | yes | yes | yes | yes |
| Each benchmark row includes baseline value, acceptable regression %, and measurement methodology | yes | yes | yes | yes |
| Runner environment metadata (hardware, OS, Rust version) is recorded with baseline | partial | yes | yes | yes |
| `conductor/quality-gates.md` includes `benchmark-regression-check` and `threshold-definition-exists` | yes | yes | yes | yes |
| `benches/regression/compare.py` exists and produces correct comparison output | yes | yes | yes | yes |
| `.github/workflows/bench-regression.yml` exists and is referenced in CI | yes | yes | yes | yes |
| `bench-regression.yml` triggers on PRs that modify `crates/kairo-ecs-core/`, `crates/kairo-ecs-state/`, `benches/`, `crates/kairo-ecs-bench/` | yes | yes | yes | yes |
| Comparison script reports benchmark name, base mean, PR mean, % change, threshold, and pass/fail | yes | yes | yes | yes |
| `threshold-definition-exists` gate fails when a benchmark lacks a threshold entry | yes | yes | yes | yes |
| `benchmark-regression-check` gate passes when supplied base/current benchmark JSON stays within thresholds | yes | yes | yes | yes |
| `benchmark-regression-check` gate fails with specific benchmark details on supplied regression JSON | yes | yes | yes | yes |
| Unknown or duplicate benchmark result IDs fail before timing claims are accepted | yes | yes | yes | yes |
| False-positive rate is below 5% on controlled runners | no | no | yes | yes |
| Maintainer can override a threshold with documented rationale | no | yes | yes | yes |
| Baseline values are updateable when intentional performance changes are accepted | no | no | yes | yes |
| Orphaned thresholds (benchmark removed but threshold remains) are detected and flagged | no | no | yes | yes |
| Gate does not block release (non-critical track) | yes | yes | yes | yes |
| Energy measurement produces valid joule data (CodeCarbon or Scaphandre integrated) | no | no | yes | yes |

## Worker 2 validation slice

| Check | Command | Result | Notes |
|---|---|---|---|
| Threshold definitions match canonical benchmark metadata | `python benches/regression/compare.py --report .tmp/track31-threshold-report.json` | pass | No missing thresholds, orphaned thresholds, owner mismatches, or measure mismatches. |
| Comparison script reports pass/fail details | `python benches/regression/compare.py --base benches/regression/sample-base.json --current benches/regression/sample-current.json --report .tmp/track31-compare-report.json` | pass | Owned local result fixtures produce per-benchmark base/current/change/threshold/status rows. |
| Positive and negative regression guard fixtures validate | `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\31-performance-regression-guard\validate-track31.ps1` | pass | Proves positive coverage/comparison fixtures pass, missing threshold fixture fails, blocking regression fixture fails with `schedule_1m_events`, and unknown benchmark ID fixture fails. |
| Benchmark smoke metadata still validates | `python benches/benchmark_smoke.py` | pass | Read-only Track 12 metadata check reports `status: ok`. |
| Workflow benchmark compile command is viable locally | `cargo bench --workspace --no-run` | fail | Windows local environment invokes Git `usr/bin/link.exe`, which fails with Win32 error 5 while creating mappings/pipes; Linux CI scaffold remains defined. |

## Current Alpha boundary

The Alpha slice validates canonical threshold coverage and the comparator logic
with checked-in positive and negative base/current JSON fixtures. It does not yet run a native
base-branch versus PR-branch benchmark pair in CI because Track 12 has not
promoted stable native benchmark output artifacts. Baseline hardware, OS, Rust
version, and multi-run statistics become mandatory when those native artifacts
land.
