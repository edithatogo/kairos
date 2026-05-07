# Risk Register: Track 31 Performance Regression Guard

Severity scale: Likelihood 1-5 x Impact 1-5. Low 1-4, Medium 5-9, High 10-16, Critical 17-25.

| Risk | Likelihood | Impact | Severity | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Benchmark noise produces false-positive regression alerts | 4 | 3 | 12 | Use mean-based comparison against percentage thresholds with controlled-runner baselines; allow rerun on controlled runner | perf-regression-agent | >3 false-positive alerts in a single release cycle |
| Baseline measurements drift due to CI runner heterogeneity | 3 | 4 | 12 | Pin controlled runner hardware for baseline; mark non-controlled-runner results as advisory only | perf-regression-agent | Controlled-runner baseline varies >5% between consecutive runs |
| A benchmark is added without a threshold definition | 2 | 3 | 6 | Gate `threshold-definition-exists` blocks PRs that add benchmarks without thresholds; `validate-track31.ps1` proves the missing-threshold failure path with a canonical fixture | perf-regression-agent | Benchmark merges without threshold definition |
| Thresholds are set too tight, causing routine CI noise | 3 | 3 | 9 | Review threshold values quarterly; allow maintainer override with documented rationale | perf-regression-agent | >5 threshold violations in a week on unchanged code |
| Thresholds are set too loose, allowing real regressions to pass | 3 | 4 | 12 | Use percentage-of-baseline thresholds with explicit categories; review on every intentional perf change | benchmark-agent | Real performance regression merges without alert |
| Comparison script produces incorrect percentage calculations | 2 | 4 | 8 | Validate against checked-in pass and fail fixtures; `validate-track31.ps1` asserts row fields, blocking regression status, and unknown-ID failure before timing claims are accepted | perf-regression-agent | Comparator fixture validation fails |
| `bench-regression.yml` times out on CI due to long benchmark runs | 3 | 3 | 9 | Set per-benchmark timeouts; sample only critical-path benchmarks on every PR, run full suite nightly | ci-agent | `bench-regression.yml` times out on main branch |
| Track 12 or Track 18 renames or removes a benchmark, orphaning threshold entries | 3 | 3 | 9 | `threshold-definition-exists` gate detects orphaned thresholds and requires cleanup | perf-regression-agent | Orphaned threshold detected by gate |
| Energy tooling unavailable on CI runner OS | 3 | 2 | 6 | Treat energy metrics as follow-up work; keep the current gate focused on timing thresholds and comparator coverage | perf-regression-agent | Energy metrics are promoted to blocking scope without runner/tooling support |
| Native benchmark outputs are not yet emitted in the comparator input format | 3 | 3 | 9 | Keep CI scaffold in threshold-validation mode until Track 12 publishes result artifacts; document supported JSON keys in `conductor/performance-thresholds.md` | perf-regression-agent + performance-agent | `bench-regression.yml` cannot consume native benchmark artifacts once Track 12 enables them |
