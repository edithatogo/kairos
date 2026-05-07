# Risk Register — 04 The Analyst: kairo-ecs-arrow Telemetry

Severity scale: Likelihood 1-5 x Impact 1-5. Low 1-4, Medium 5-9, High 10-16, Critical 17-25.

| Risk | Likelihood | Impact | Severity | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Arrow schema evolution breaking IPC compatibility | 3 | 5 | 15 | Schema versioning with backward-compatibility policy; IPC round-trip tests in CI | analyst-agent | IPC round-trip test fails after schema change |
| Zero-copy claims without lifetime documentation | 3 | 4 | 12 | Explicit lifetime contracts in docs; Miri validation on all zero-copy paths | analyst-agent | Use-after-free or dangling pointer reported in consumer code |
| Serialization perf regression without benchmarks | 4 | 3 | 12 | Benchmark serialization throughput with regression thresholds per CI run | analyst-agent | Serialization benchmark regresses >20% |
| Apache Arrow version incompatibility across bindings | 3 | 4 | 12 | Pin minimum Arrow version; validate IPC interop across binding tiers | analyst-agent | Any binding tier fails IPC interop test with pinned Arrow version |
| OTel SDK version drift breaks export pipeline | 3 | 3 | 9 | Pin `opentelemetry` and `opentelemetry-otlp` crate versions; test on CI with pinned OTLP collector | arrow-agent | OTel export test fails on dependency update |
