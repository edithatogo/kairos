# Handoff — 02 The Bridge: kairo-ecs-ffi, UniFFI & Diplomat

Last verified: 2026-05-08

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
cargo +stable-x86_64-pc-windows-gnu check --tests -p kairo-ecs-ffi -p kairo-ecs-uniffi -p kairo-ecs-diplomat
cargo +stable-x86_64-pc-windows-gnu fmt --check -p kairo-ecs-ffi -p kairo-ecs-uniffi -p kairo-ecs-diplomat
cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-ffi -p kairo-ecs-uniffi -p kairo-ecs-diplomat
cargo +stable-x86_64-pc-windows-gnu metadata --no-deps --format-version 1
```

Not rerun for this closeout because the repo already has unrelated local Conductor and non-Track-02 edits:

```text
cargo fmt --all --check
```

## Follow-up issues

- Replace placeholder telemetry JSON with the Track 04 Arrow IPC bridge output.
- Add generated UniFFI/Diplomat golden output once those generators are pinned in CI.
- Replace the wrapper-anchor facades with generated UniFFI/Diplomat outputs once those generators are pinned.
## Phase closeout evidence

Track 02 advanced from `In Progress` to `In Review` on 2026-05-08.

- `$conductor-review` result: the existing Track 02 implementation satisfies the implementation-slice gates for the stable C ABI bridge plus UniFFI and Diplomat wrapper anchors.
- Accepted fixes: status and evidence closeout only; no additional in-scope bridge code changes were required after focused validation passed.
- Deferred or blocked fixes: generated UniFFI/Diplomat golden outputs remain deferred until generators are pinned; Track 04 Arrow IPC telemetry remains future integration; native package publication remains Track 15/20 scope.
- Validation commands:
  - `cargo +stable-x86_64-pc-windows-gnu check --tests -p kairo-ecs-ffi -p kairo-ecs-uniffi -p kairo-ecs-diplomat` — passed.
  - `cargo +stable-x86_64-pc-windows-gnu fmt --check -p kairo-ecs-ffi -p kairo-ecs-uniffi -p kairo-ecs-diplomat` — passed.
  - `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-ffi -p kairo-ecs-uniffi -p kairo-ecs-diplomat` — passed, 23 tests.
  - `cargo +stable-x86_64-pc-windows-gnu metadata --no-deps --format-version 1` — passed.
- Cleanup state: local working tree was already dirty with prior Conductor closeout changes outside Track 02; no unrelated changes were reverted.
- Commit SHA / pushed ref: `5dd1937566898b2e028ac61dab1e9dd173e6d919` on `origin/main` is the current pushed base for this local closeout pass.
- Strict cleanup gate: rerun `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after committing the local closeout batch.
- Next-phase decision: Track 02 is `In Review`; reviewer signoff is still required before moving the stable C ABI bridge to `Done`.
