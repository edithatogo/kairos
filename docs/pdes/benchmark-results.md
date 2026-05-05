# PDES Benchmark Results

No scaling run has been performed yet. The benchmark scaffold is present under
`benches/pdes/`, but throughput targets must not be marked complete until run on
controlled hardware with at least four physical cores.

Required scenarios:

- LP counts: 4, 8, 16, 32.
- Workloads: entity-spawn-heavy, event-heavy, query-heavy, mixed.
- Metrics: ticks per second, final-state parity, GVT progression rate.

Validation command for the current scaffold:

```powershell
cargo test --manifest-path crates/kairo-ecs-pdes/Cargo.toml --features pdes
```
