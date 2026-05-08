# Risk Register: Track 18 Comparative Benchmarks & Reproducibility

Severity scale: Likelihood 1-5 x Impact 1-5. Low 1-4, Medium 5-9, High 10-16, Critical 17-25.

| Risk | Likelihood | Impact | Severity | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Benchmark methodology not comparable to ecosystem baselines | 4 | 4 | 16 | Baseline against published DES benchmarks (PHOLD, DEVStone) with identical model configuration | benchmark-agent | Any published benchmark uses non-comparable methodology |
| Hardware variance obscures results | 4 | 4 | 16 | Pin runner class in CI; record CPU model, frequency, and memory in result metadata | ci-agent | Same benchmark varies >10% across runs on pinned runner |
| Reproducibility metadata missing | 4 | 4 | 16 | Gate seed, scenario definition, runner env details, raw output, toolchain, feature flags, and baseline version through `benches/raw-results-policy.json` and `python benches/benchmark_reproducibility.py` | benchmark-agent | Any published result missing seed or runner environment metadata |
| Benchmark results misinterpreted as marketing claims | 3 | 3 | 9 | Include a "How to read these results" section with caveats on every benchmark page | docs-agent | Benchmark page published without caveats section |
| Runner environment drift over time | 3 | 3 | 9 | Version-lock benchmark harness; archive results with runner-version and dependency-hash metadata | benchmark-agent | Historical comparison shows >5% drift without code change |
| Fixture or smoke metadata drift breaks replay | 3 | 4 | 12 | Gate Track 18 docs with `python benches/benchmark_smoke.py` and `python benches/benchmark_reproducibility.py` so ready fixture IDs, source files, canonical scenarios, owners, and smoke scales stay aligned | benchmark-agent | Either metadata command fails or a published comparison names a non-ready fixture |
