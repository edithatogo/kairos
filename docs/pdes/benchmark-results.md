# PDES Benchmark Results

Local deterministic scaling smoke evidence now exists for the required 4, 8,
16, and 32 LP configurations through `scaling_smoke_samples` in
`crates/kairo-ecs-pdes/src/lib.rs`. The smoke run compares the sequential oracle
with the partitioned reference, checks final-state parity, confirms GVT sample
coverage, and records remote-event/null-message counts.

No hardware-speedup or hardware-parity claim is made in this slice.

Required scenarios:

- LP counts: 4, 8, 16, 32.
- Workloads: entity-spawn-heavy, event-heavy, query-heavy, mixed.
- Metrics: ticks per second, final-state parity, GVT progression rate.

Current local smoke evidence:

| LPs | Ticks | Entities/LP | Expected evidence |
|---:|---:|---:|---|
| 4 | 256 | 4 | final-state parity, GVT samples, remote events, null messages |
| 8 | 256 | 4 | final-state parity, GVT samples, remote events, null messages |
| 16 | 256 | 4 | final-state parity, GVT samples, remote events, null messages |
| 32 | 256 | 4 | final-state parity, GVT samples, remote events, null messages |

The deterministic smoke suite is not a throughput benchmark. Throughput targets
remain unclaimed until run on controlled hardware with at least four physical
cores.

Validation command for the current scaffold:

```powershell
cargo test --manifest-path crates/kairo-ecs-pdes/Cargo.toml --features pdes
```
