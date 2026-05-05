# Handoff — 01 The Heart: kairo-ecs-core & kairo-ecs-state

## Summary

Track 01 now has a minimal implemented core slice anchored by `lanes.md`, the shared conformance fixtures, and package-focused cargo gates. The current implementation surface is `crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, and `crates/kairo-ecs-rng`.

## Files changed

- `crates/kairo-ecs-types/src/lib.rs`
- `crates/kairo-ecs-core/src/lib.rs`
- `crates/kairo-ecs-state/src/lib.rs`
- `crates/kairo-ecs-rng/src/lib.rs`
- `conductor/tracks/01-heart-kairo-ecs-core-state/handoff.md`
- `conductor/tracks/01-heart-kairo-ecs-core-state/test-matrix.md`

## Contracts consumed

The track consumes the scheduler, state, types, and RNG contracts that are staged through `lanes.md` and the conformance fixture set.

## Contracts changed

The next contract surface is the deterministic ordering and replay behavior in `crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, and `crates/kairo-ecs-rng`.

## Tests added

- Unit tests cover fixed-tick time overflow, scheduler ordering, cancellation, bounded runs, state lifecycle, and deterministic RNG replay.
- This review pass added regression coverage that cancellation rejects unknown or already-dispatched IDs and does not let cancelled future events force a false limit outcome.
- The state crate now exposes a deterministic `WorldSnapshot` over live entities. Snapshot entities are sorted by `(index, generation)` so downstream Track 04/05 consumers do not inherit nondeterministic `HashSet` iteration order.
- This pass hardened `ComponentStore` for generational handles: duplicate inserts replace the existing row, stale generations cannot read or remove a component at the same index, and a newer generation supersedes the stale indexed row deterministically.

## Validation run

- `cargo fmt --package kairo-ecs-state`
- `cargo fmt --package kairo-ecs-types --package kairo-ecs-core --package kairo-ecs-state --package kairo-ecs-rng --check`
- `cargo check --tests -p kairo-ecs-types -p kairo-ecs-core -p kairo-ecs-state -p kairo-ecs-rng`
- `cargo check -p kairo-ecs-state --tests` covers the deterministic `WorldSnapshot` API without linking a Windows test executable.
- `cargo clippy -p kairo-ecs-state --all-targets -- -D warnings`
- `cargo test -p kairo-ecs-state` was attempted but blocked by the local Windows linker resolving `link.exe` to Git for Windows (`C:\Users\60217257\scoop\apps\git\current\usr\bin\link.exe`), which failed with `couldn't create signal pipe, Win32 error 5`.
- 2026-05-06 linker follow-up: `rustc -Vv` reports host `x86_64-pc-windows-msvc`, `where link` resolves only Git for Windows `link.exe`, `where cl` finds no MSVC compiler on PATH, and `vswhere -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64` returned no installation path. `cargo test -p kairo-ecs-state` still fails before running tests because Rust invokes the Git `link.exe`; this run returned exit code `0xc0000142` with the same Git `link.exe` signal-pipe failure. `cargo check --tests -p kairo-ecs-state` remains the local validation gate until an MSVC linker environment is available.
- Prior validation also recorded the same local linker blocker for `cargo test -p kairo-ecs-core`.

## Known risks

The main risk is drifting out of sync with `lanes.md` if the implementation slices change without updating the fixture and validation docs.

## Integration notes

Next implementation step: bind the implemented scheduler/state/RNG slice to shared conformance fixtures and keep the fixture manifest aligned as each lane closes.
