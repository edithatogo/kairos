# PDES Validation Evidence

Track 34 keeps PDES validation conservative and local to `kairo-ecs-pdes`.
The current validator does not claim scheduler integration or real speedup.

## Local Validator

`validate_conservative_pdes()` runs one or more deterministic workloads through
the sequential reference and partitioned reference, then returns explicit
evidence for:

- final-state parity against the sequential oracle;
- monotonic GVT history;
- final GVT reaching the requested tick;
- non-empty remote-event and null-message traffic;
- deadlock smoke completion for the full tick count.

The long stress fixture remains `deadlock_stress_report()`: 8 LPs, 10,000
ticks, and protocol traffic on every tick.

## Focused Commands

```powershell
cargo check --manifest-path crates/kairo-ecs-pdes/Cargo.toml --features pdes
cargo check --manifest-path crates/kairo-ecs-pdes/Cargo.toml --features pdes --tests
cargo fmt --manifest-path crates/kairo-ecs-pdes/Cargo.toml -- --check
cargo test --manifest-path crates/kairo-ecs-pdes/Cargo.toml --features pdes
```

On the current Windows workstation, the first three checks pass. Runtime test
execution is blocked before tests start because the MSVC test binary links via
Git's `usr\bin\link.exe`, which fails with Win32 error 5. The `rust-lld`
fallback also links far enough to prove compilation but cannot find Windows SDK
import libraries such as `kernel32.lib`.

## Scope Boundary

This evidence validates the PDES scaffold and protocol fixtures only. It does
not enable the `pdes` feature by default, modify the core scheduler, or replace
Track 12 benchmark harness ownership.
