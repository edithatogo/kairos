# Risk Register: Track 31 Performance Regression Guard

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Benchmark noise produces false-positive regression alerts | 4 | 3 | 12 | Use statistical comparison (mean + t-test) with minimum 5-run samples; allow re-run on controlled runner | perf-regression-agent | >3 false-positive alerts in a single release cycle |
| Baseline measurements drift due to CI runner heterogeneity | 3 | 4 | 12 | Pin controlled runner hardware for baseline; mark non-controlled-runner results as advisory only | perf-regression-agent | Controlled-runner baseline varies >5% between consecutive runs |
| A benchmark is added without a threshold definition | 3 | 3 | 9 | Gate `threshold-definition-exists` blocks PRs that add benchmarks without thresholds | perf-regression-agent | Benchmark merges without threshold definition |
| Thresholds are set too tight, causing routine CI noise | 3 | 3 | 9 | Review threshold values quarterly; allow maintainer override with documented rationale | perf-regression-agent | >5 threshold violations in a week on unchanged code |
| Thresholds are set too loose, allowing real regressions to pass | 3 | 4 | 12 | Use percentage-of-baseline thresholds with explicit categories; review on every intentional perf change | benchmark-agent | Real performance regression merges without alert |
| Comparison script produces incorrect percentage calculations | 2 | 4 | 8 | Unit-test the comparison logic; validate against manual calculation on a known delta | perf-regression-agent | Comparison unit test fails |
| `bench-regression.yml` times out on CI due to long benchmark runs | 3 | 3 | 9 | Set per-benchmark timeouts; sample only critical-path benchmarks on every PR, run full suite nightly | ci-agent | `bench-regression.yml` times out on main branch |
| Track 12 or Track 18 renames or removes a benchmark, orphaning threshold entries | 3 | 3 | 9 | `threshold-definition-exists` gate detects orphaned thresholds and requires cleanup | perf-regression-agent | Orphaned threshold detected by gate |
| Energy tooling unavailable on CI runner OS | 3 | 2 | 6 | Use `CodeCarbon` as primary, `Scaphandre` as fallback; run energy measurements on Linux runners only; document platform gaps | perf-regression-agent | Energy data missing from >1 consecutive benchmark report |
