# Test Matrix: Track 31 Performance Regression Guard

| Check | Alpha | Beta | RC | 1.0 |
|---:|---:|---:|---:|---:|
| Track docs exist and render cleanly | yes | yes | yes | yes |
| `conductor/performance-thresholds.md` exists and lists every active benchmark | yes | yes | yes | yes |
| Each benchmark row includes baseline value, acceptable regression %, and measurement methodology | yes | yes | yes | yes |
| Runner environment metadata (hardware, OS, Rust version) is recorded with baseline | yes | yes | yes | yes |
| `conductor/quality-gates.md` includes `benchmark-regression-check` and `threshold-definition-exists` | yes | yes | yes | yes |
| `benches/regression/compare.py` exists and produces correct comparison output | yes | yes | yes | yes |
| `.github/workflows/bench-regression.yml` exists and is referenced in CI | yes | yes | yes | yes |
| `bench-regression.yml` triggers on PRs that modify `crates/kairo-ecs-core/`, `crates/kairo-ecs-state/`, `benches/`, `crates/kairo-ecs-bench/` | no | yes | yes | yes |
| Comparison script reports benchmark name, base mean, PR mean, % change, threshold, and pass/fail | no | yes | yes | yes |
| `threshold-definition-exists` gate fails when a benchmark lacks a threshold entry | no | yes | yes | yes |
| `benchmark-regression-check` gate passes when all benchmarks are within thresholds | no | yes | yes | yes |
| `benchmark-regression-check` gate fails with specific benchmark details on regression | no | yes | yes | yes |
| False-positive rate is below 5% on controlled runners | no | no | yes | yes |
| Maintainer can override a threshold with documented rationale | no | yes | yes | yes |
| Baseline values are updateable when intentional performance changes are accepted | no | no | yes | yes |
| Orphaned thresholds (benchmark removed but threshold remains) are detected and flagged | no | no | yes | yes |
| Gate does not block release (non-critical track) | yes | yes | yes | yes |
| Energy measurement produces valid joule data (CodeCarbon or Scaphandre integrated) | no | no | yes | yes |
