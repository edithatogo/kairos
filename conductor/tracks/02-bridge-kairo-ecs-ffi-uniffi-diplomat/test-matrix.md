# Test Matrix — 02 The Bridge: kairo-ecs-ffi, UniFFI & Diplomat

Last verified: 2026-05-08

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

## Current Track 02 validation — 2026-05-08

- `cargo +stable-x86_64-pc-windows-gnu check --tests -p kairo-ecs-ffi -p kairo-ecs-uniffi -p kairo-ecs-diplomat` — pass.
- `cargo +stable-x86_64-pc-windows-gnu fmt --check -p kairo-ecs-ffi -p kairo-ecs-uniffi -p kairo-ecs-diplomat` — pass.
- `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-ffi -p kairo-ecs-uniffi -p kairo-ecs-diplomat` — pass, 23 tests.
- `cargo +stable-x86_64-pc-windows-gnu metadata --no-deps --format-version 1` — pass.
- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` — pass.
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` — pass.
- `pwsh -NoProfile -File scripts/validate_conductor_setup.ps1 -SkipCargo` — pass.
- `pwsh -NoProfile -File scripts/validate_track_coverage.ps1 -SkipCargo` — pass.
- `cargo fmt --all --check` was not rerun for this closeout because the repo already has unrelated local Conductor and non-Track-02 edits in the working tree. The focused Track 02 formatting gate passed.

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
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
