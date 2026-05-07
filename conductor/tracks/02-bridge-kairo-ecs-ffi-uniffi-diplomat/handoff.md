# Handoff — 02 The Bridge: kairo-ecs-ffi, UniFFI & Diplomat

Last verified: 2026-05-07

## Summary

Track 02 now has a minimal real bridge slice across the stable C ABI and the two generated-wrapper anchor crates. The C ABI remains the source of truth; UniFFI and Diplomat expose dependency-light Rust facades over the same handle lifecycle and status-code surface.

## Files changed

- `crates/kairo-ecs-ffi/src/lib.rs`
- `crates/kairo-ecs-ffi/tests/ffi_integration.rs`
- `crates/kairo-ecs-uniffi/src/lib.rs`
- `crates/kairo-ecs-diplomat/src/lib.rs`
- `conductor/tracks.yaml`
- `conductor/tracks/02-bridge-kairo-ecs-ffi-uniffi-diplomat/spec.md`
- `conductor/tracks/02-bridge-kairo-ecs-ffi-uniffi-diplomat/test-matrix.md`
- `conductor/tracks/02-bridge-kairo-ecs-ffi-uniffi-diplomat/handoff.md`

## Contracts consumed

`crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, `crates/kairo-ecs-rng`, and the shared Track 12 fixture manifest.

## Contracts changed

No shared contract files were changed. The implementation now enforces the existing FFI contract more directly: exported ABI functions run through a `catch_unwind` boundary, lifecycle double-free returns `KAIRO_ECS_ERR_ALREADY_FREED`, `kairo_ecs_step`, `kairo_ecs_run_for`, `kairo_ecs_run_until`, and `kairo_ecs_run_until_or_for` update bridge stats and handle ownership consistently, and the canonical header is checked against a deterministic generated-header fixture.

## Tests added

- FFI lifecycle coverage for create/free/double-free.
- Panic-boundary unit coverage returning `KAIRO_ECS_ERR_PANIC`.
- Header-diff unit coverage comparing `include/kairo_ecs.h` with the generated ABI text.
- Integration coverage for `schedule_after` using current simulation time.
- Integration coverage for `run_until_or_for` enforcing both the time and event-count bounds.
- UniFFI facade smoke coverage for schedule/step/stats/close.
- Diplomat facade smoke coverage for schedule/step/current-time/close.

## Known risks

The focused Track 02 test suite now executes on this host. Full workspace formatting remains blocked by unrelated formatting drift outside Track 02-owned paths, recorded in `test-matrix.md`.

## Integration notes

Validation run:

```text
cargo check --tests -p kairo-ecs-ffi -p kairo-ecs-uniffi -p kairo-ecs-diplomat
cargo fmt --check -p kairo-ecs-ffi -p kairo-ecs-uniffi -p kairo-ecs-diplomat
cargo test -p kairo-ecs-ffi -p kairo-ecs-uniffi -p kairo-ecs-diplomat
```

Attempted but blocked by unrelated workspace formatting drift:

```text
cargo fmt --all --check
```

## Follow-up issues

- Replace placeholder telemetry JSON with the Track 04 Arrow IPC bridge output.
- Add generated UniFFI/Diplomat golden output once those generators are pinned in CI.
- Promote Track 02 toward `In Review` only after affected binding tracks confirm the ABI shape is sufficient.
## Phase closeout evidence

Pending for the next actual phase closeout. Before this track advances, record `$conductor-review` findings, accepted fixes, deferred or blocked fixes, validation commands, cleanup state, commit SHA or explicit push blocker, pushed ref, strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` result, and next-phase decision here.