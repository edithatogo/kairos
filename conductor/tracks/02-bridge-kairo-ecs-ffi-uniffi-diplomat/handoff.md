# Handoff — 02 The Bridge: kairo-ecs-ffi, UniFFI & Diplomat

## Summary

Track 02 now has a minimal real R2 bridge slice across the stable C ABI and the two generated-wrapper anchor crates. The C ABI remains the source of truth; UniFFI and Diplomat now expose dependency-light Rust facades over the same handle lifecycle and status-code surface.

## Files changed

- `crates/kairo-ecs-ffi/src/lib.rs`
- `crates/kairo-ecs-uniffi/src/lib.rs`
- `crates/kairo-ecs-diplomat/src/lib.rs`
- `conductor/tracks/02-bridge-kairo-ecs-ffi-uniffi-diplomat/handoff.md`

## Contracts consumed

`crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, `crates/kairo-ecs-rng`, and the shared Track 12 fixture manifest.

## Contracts changed

No shared contract files were changed. The implementation now enforces the existing FFI contract more directly: exported ABI functions run through a `catch_unwind` boundary, lifecycle double-free returns `KAIRO_ECS_ERR_ALREADY_FREED`, `kairo_ecs_step` updates bridge stats consistently, and the canonical header is checked against a deterministic generated-header fixture.

## Tests added

- FFI lifecycle coverage for create/free/double-free.
- Panic-boundary unit coverage returning `KAIRO_ECS_ERR_PANIC`.
- Header-diff unit coverage comparing `include/kairo_ecs.h` with the generated ABI text.
- UniFFI facade smoke coverage for schedule/step/stats/close.
- Diplomat facade smoke coverage for schedule/step/current-time/close.

## Known risks

`cargo test` could not execute on this machine because the Windows environment resolves `link.exe` to Git for Windows (`C:\Users\60217257\scoop\apps\git\current\usr\bin\link.exe`), and the bundled `rust-lld` retry lacks the Windows SDK import libraries (`kernel32.lib`, `ntdll.lib`, `userenv.lib`, `ws2_32.lib`, `dbghelp.lib`). Test targets do compile under `cargo check --tests`, but executable test runs need the MSVC linker/SDK path fixed.

## Integration notes

Validation run:

```text
cargo fmt --package kairo-ecs-ffi --package kairo-ecs-uniffi --package kairo-ecs-diplomat --check
cargo check -p kairo-ecs-ffi -p kairo-ecs-uniffi -p kairo-ecs-diplomat
cargo check --tests -p kairo-ecs-ffi -p kairo-ecs-uniffi -p kairo-ecs-diplomat
cargo metadata --no-deps --format-version 1
where.exe link
```

Attempted but blocked by host linker setup:

```text
cargo test -p kairo-ecs-ffi -p kairo-ecs-uniffi -p kairo-ecs-diplomat
$env:RUSTFLAGS='-C linker=rust-lld'; cargo test -p kairo-ecs-ffi -p kairo-ecs-uniffi -p kairo-ecs-diplomat
```
