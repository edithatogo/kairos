# PDES Benchmarks

This directory is reserved for Track 34 scaling benchmarks.

Planned matrix:

- LP counts: 4, 8, 16, 32.
- Workloads: entity-spawn-heavy, event-heavy, query-heavy, mixed.
- Required checks: sequential final-state parity, ticks per second, GVT
  progression rate.

No benchmark result should be marked complete until the suite is run on
controlled hardware with at least four physical cores.

Current validation:

```powershell
cargo test --manifest-path crates/kairo-ecs-pdes/Cargo.toml --features pdes
```
