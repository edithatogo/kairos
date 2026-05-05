# Risk Register: Track 18 Comparative Benchmarks & Reproducibility

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Benchmark methodology not comparable to ecosystem baselines | 4 | 4 | 16 | Baseline against published DES benchmarks (PHOLD, DEVStone) with identical model configuration | benchmark-agent | Any published benchmark uses non-comparable methodology |
| Hardware variance obscures results | 4 | 4 | 16 | Pin runner class in CI; record CPU model, frequency, and memory in result metadata | ci-agent | Same benchmark varies >10% across runs on pinned runner |
| Reproducibility metadata missing | 4 | 4 | 16 | Publish seed, scenario definition, runner env details, and raw output with every result | benchmark-agent | Any published result missing seed or runner environment metadata |
| Benchmark results misinterpreted as marketing claims | 3 | 3 | 9 | Include a "How to read these results" section with caveats on every benchmark page | docs-agent | Benchmark page published without caveats section |
| Runner environment drift over time | 3 | 3 | 9 | Version-lock benchmark harness; archive results with runner-version and dependency-hash metadata | benchmark-agent | Historical comparison shows >5% drift without code change |
