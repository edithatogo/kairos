# Test Matrix — 02 The Bridge: kairo-ecs-ffi, UniFFI & Diplomat

## Required tests

### Track gates (from tracks.yaml)

- `ffi-lifecycle-tests` — C test that creates, uses, and frees an engine and detects leaks/double-free.
- `panic-boundary-tests` — test that triggers a panic inside Rust and confirms the FFI boundary catches it with `KAIRO_ECS_ERR_PANIC`.
- `header-diff` — diff check between the generated C header and the canonical `include/kairo_ecs.h`.

### General workspace checks

- `cargo metadata --no-deps --format-version 1`
- `cargo test --workspace --all-features`
- `cargo fmt --all --check`
- Bridge fixture parity checks once the exported surface exists.
- Header or generated-wrapper smoke tests for the binding outputs once the wrapper crates land.
- Docs build if this track changes the bridge docs.
- Package dry-run if this track changes package metadata.

## CI commands

```bash
test -f conductor/tracks/02-bridge-kairo-ecs-ffi-uniffi-diplomat/spec.md
test -f conductor/tracks/02-bridge-kairo-ecs-ffi-uniffi-diplomat/plan.md
test -f conductor/tracks/02-bridge-kairo-ecs-ffi-uniffi-diplomat/handoff.md
cargo metadata --no-deps --format-version 1
cargo test --workspace --all-features
cargo fmt --all --check
pwsh -NoProfile -File scripts/validate_conductor_setup.ps1 -SkipCargo
pwsh -NoProfile -File scripts/validate_track_coverage.ps1 -SkipCargo
```
