# Risk Register: Track 31 Performance Regression Guard

| Risk | Likelihood | Impact | Mitigation | Owner |
|---|---:|---:|---|---|
| Benchmark noise produces false-positive regression alerts | High | Medium | Use statistical comparison (mean + t-test) with minimum 5-run samples; allow re-run on controlled runner | perf-regression-agent |
| Baseline measurements drift due to CI runner heterogeneity | Medium | High | Pin controlled runner hardware for baseline; mark non-controlled-runner results as advisory only | perf-regression-agent |
| A benchmark is added without a threshold definition | Medium | Medium | Gate `threshold-definition-exists` blocks PRs that add benchmarks without thresholds | perf-regression-agent |
| Thresholds are set too tight, causing routine CI noise | Medium | Medium | Review threshold values quarterly; allow maintainer override with documented rationale | perf-regression-agent |
| Thresholds are set too loose, allowing real regressions to pass | Medium | High | Use percentage-of-baseline thresholds with explicit categories; review on every intentional perf change | benchmark-agent |
| Comparison script produces incorrect percentage calculations | Low | High | Unit-test the comparison logic; validate against manual calculation on a known delta | perf-regression-agent |
| `bench-regression.yml` times out on CI due to long benchmark runs | Medium | Medium | Set per-benchmark timeouts; sample only critical-path benchmarks on every PR, run full suite nightly | ci-agent |
| Track 12 or Track 18 renames or removes a benchmark, orphaning threshold entries | Medium | Medium | `threshold-definition-exists` gate detects orphaned thresholds and requires cleanup | perf-regression-agent |
