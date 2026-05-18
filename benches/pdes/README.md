# PDES Benchmarks

This directory is reserved for Track 34 scaling benchmarks. The current
repository evidence is a deterministic benchmark-smoke harness in
`crates/kairo-ecs-pdes/src/lib.rs`, not a wall-clock throughput benchmark.

Planned matrix:

- LP counts: 4, 8, 16, 32.
- Workloads: entity-spawn-heavy, event-heavy, query-heavy, mixed.
- Required checks: sequential final-state parity, ticks per second, GVT
  progression rate.

Benchmark-smoke coverage is complete for 4/8/16/32 LP logical configurations
when `scaling_smoke_samples(&[4, 8, 16, 32])` passes. Hardware speedup results
must not be marked complete until the suite is run on controlled hardware with
at least four physical cores.

Current validation:

```powershell
cargo test --manifest-path crates/kairo-ecs-pdes/Cargo.toml --features pdes
```

Until Track 12 integrates a real benchmark target, PDES evidence is limited to
the local validator in `kairo-ecs-pdes`: final-state parity, GVT progression,
protocol traffic counts, 4/8/16/32 LP benchmark-smoke samples, Time Warp spike
documentation, and deadlock-smoke completion. Do not publish speedup claims from
this directory.
