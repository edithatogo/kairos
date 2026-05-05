# Risk Register — 04 The Analyst: kairo-ecs-arrow Telemetry

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Arrow schema evolution breaking IPC compatibility | 3 | 5 | 15 | Schema versioning with backward-compatibility policy; IPC round-trip tests in CI | analyst-agent | IPC round-trip test fails after schema change |
| Zero-copy claims without lifetime documentation | 3 | 4 | 12 | Explicit lifetime contracts in docs; Miri validation on all zero-copy paths | analyst-agent | Use-after-free or dangling pointer reported in consumer code |
| Serialization perf regression without benchmarks | 4 | 3 | 12 | Benchmark serialization throughput with regression thresholds per CI run | analyst-agent | Serialization benchmark regresses >20% |
| Apache Arrow version incompatibility across bindings | 3 | 4 | 12 | Pin minimum Arrow version; validate IPC interop across binding tiers | analyst-agent | Any binding tier fails IPC interop test with pinned Arrow version |
