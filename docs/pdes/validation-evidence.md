# PDES Validation Evidence

Track 34 keeps PDES validation conservative and local to `kairo-ecs-pdes`.
The current validator does not claim scheduler integration or real speedup.
No hardware-speedup or hardware-parity claim is made in this slice.

## Local Validator

`validate_conservative_pdes()` runs one or more deterministic workloads through
the sequential reference and partitioned reference, then returns explicit
evidence for:

- final-state parity against the sequential oracle;
- monotonic GVT history;
- final GVT reaching the requested tick;
- non-empty remote-event and null-message traffic;
- deadlock smoke completion for the full tick count.

Additional transport/lifecycle checks now recorded in this slice:

- `PartitionPlan::from_entities` sorts generational entity IDs deterministically,
  assigns them round-robin across LPs, records owner lookups, and rejects zero
  LPs, zero lookahead, empty entity sets, and duplicate entity IDs.
- LP registration rejects duplicate LP IDs, mismatched segment IDs, self-neighbor
  declarations, and duplicate neighbor entries at `add_lp` time.
- transport sends/receives are strict; sending or receiving against unknown LP
  IDs fails with `TransportError`, and send envelopes with an unknown source or
  mismatched embedded destination are rejected before queueing.
- stale null-message safe times never move an LP backwards; the scheduler clamps
  the safe processing target to at least the LP's current local time.
- scheduler step advancement is fallible and propagates routing/registration failures
  to testable local evidence.

The long stress fixture remains `deadlock_stress_report()`: 8 LPs, 10,000
ticks, and protocol traffic on every tick.

## Focused Commands

```powershell
cargo check --manifest-path crates/kairo-ecs-pdes/Cargo.toml --features pdes
cargo check --manifest-path crates/kairo-ecs-pdes/Cargo.toml --features pdes --tests
cargo fmt --manifest-path crates/kairo-ecs-pdes/Cargo.toml -- --check
cargo test --manifest-path crates/kairo-ecs-pdes/Cargo.toml --features pdes
```

The Track 34 validation wrapper runs the compile checks by default and runs
runtime tests with `-RunTests` through the GNU stable toolchain where available.

## Scope Boundary

This evidence validates the PDES scaffold and protocol fixtures only. It does
not enable the `pdes` feature by default, modify the core scheduler, or replace
Track 12 benchmark harness ownership.
