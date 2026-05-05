# Test Matrix — 01 The Heart: kairo-ecs-core & kairo-ecs-state

## Required tests

- `cargo fmt --all --check`
- `cargo clippy -p kairo-ecs-types -p kairo-ecs-core -p kairo-ecs-state -p kairo-ecs-rng --all-targets -- -D warnings`
- `cargo test -p kairo-ecs-types`
- `cargo test -p kairo-ecs-core`
- `cargo test -p kairo-ecs-state`
- `cargo test -p kairo-ecs-rng`
- `cargo test --workspace`
- `cargo check --tests -p kairo-ecs-types -p kairo-ecs-core -p kairo-ecs-state -p kairo-ecs-rng` is the local Windows linker-safe gate when executable test linking is blocked.
- `cargo check --tests -p kairo-ecs-state` must cover the deterministic `WorldSnapshot` API consumed by Track 05 visualization without linking a test executable.
- Local Windows linker blocker verified 2026-05-06: `rustc -Vv` uses host `x86_64-pc-windows-msvc`, but PATH resolves `link.exe` to Git for Windows (`C:\Users\60217257\scoop\apps\git\current\usr\bin\link.exe`), `where cl` finds no MSVC compiler, and `vswhere -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64` returns no VC tools installation path. On this machine, use the `cargo check --tests` gate unless a proper MSVC build-tools environment is explicitly activated.
- Fixture presence check for `conformance/fixtures/deterministic_ordering.json`, `conformance/fixtures/cancellation.json`, and `conformance/fixtures/rng_replay.json`
- `scripts/validate_conductor_setup.ps1` and `scripts/validate_track_coverage.ps1` both succeed

## CI commands

```bash
cargo fmt --all --check
cargo clippy -p kairo-ecs-types -p kairo-ecs-core -p kairo-ecs-state -p kairo-ecs-rng --all-targets -- -D warnings
cargo test -p kairo-ecs-types
cargo test -p kairo-ecs-core
cargo test -p kairo-ecs-state
cargo test -p kairo-ecs-rng
cargo test --workspace
pwsh -NoProfile -File scripts/validate_conductor_setup.ps1 -SkipCargo
pwsh -NoProfile -File scripts/validate_track_coverage.ps1 -SkipCargo
test -f conformance/fixtures/deterministic_ordering.json
test -f conformance/fixtures/cancellation.json
test -f conformance/fixtures/rng_replay.json
```

## Review-hardening coverage

The Track 01 scheduler tests must include cancellation regression coverage for unknown IDs, already-dispatched IDs, duplicate cancellation, and cancelled future events that should not force a false `LimitReached` after active work finishes.

The Track 01 state tests must include deterministic snapshot ordering for live entities so downstream Arrow and visualization consumers do not depend on `HashSet` iteration order.

The Track 01 state component-store tests must include generational handle coverage: same-entity replacement must not duplicate dense rows, stale generations must not read or remove the current component for the same index, and a newer generation must supersede the stale indexed row deterministically.
