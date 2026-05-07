# Handoff — 01 The Heart: kairo-ecs-core & kairo-ecs-state

Cleanup date: 2026-05-08

## Summary

Track 01 currently contains the deterministic heart slice that is already implemented in the workspace:

- `kairo-ecs-types` provides the time, ID, request, and outcome types.
- `kairo-ecs-core` provides the deterministic scheduler with ordering, cancellation, bounded runs, scheduler stats, and replay recording.
- `kairo-ecs-core` now provides a pure Rust `SchedulerFacade` plus stable status/result wrappers for Track 01E facade readiness.
- `kairo-ecs-state` provides the entity world, deterministic snapshot ordering, and generational component storage.
- `kairo-ecs-rng` provides run-seed derivation and deterministic entity streams.

The checked-in implementation also includes the Track 01E pure Rust facade slice and core-owned scheduler stats collection. This cleanup keeps the Track 01 docs aligned with that implementation state and lane ownership.

## Files changed

- `conductor/tracks/01-heart-kairo-ecs-core-state/agent-contract.md`
- `conductor/tracks/01-heart-kairo-ecs-core-state/risk-register.md`
- `conductor/tracks/01-heart-kairo-ecs-core-state/test-matrix.md`
- `conductor/tracks/01-heart-kairo-ecs-core-state/handoff.md`
- `crates/kairo-ecs-core/src/lib.rs`
- `crates/kairo-ecs-core/tests/integration.rs`
- `crates/kairo-ecs-state/src/lib.rs`

## Lane ownership

The lane map in `lanes.md` remains the execution boundary:

- 01A Types and time: `contracts-agent`
- 01B Scheduler: `core-scheduler-agent`
- 01C State: `ecs-agent`
- 01D RNG: `rng-agent`
- 01E Facade readiness: `core-scheduler-agent` + `ffi-agent`

## Validation run

Use the local gates in `test-matrix.md` for this track. The reliable local fallback on this machine is:

- `cargo check --tests -p kairo-ecs-state`

Full executable tests remain the expected gate when a working MSVC linker environment is available. On this machine, prior full-test attempts have been sensitive to `link.exe` resolution, so `cargo check --tests` remains the documented local fallback until the shell/toolchain is verified.

The Track 01E facade validation surface is:

- `cargo test -p kairo-ecs-core`
- `cargo test -p kairo-ecs-types`
- `cargo test -p kairo-ecs-state`
- `cargo test -p kairo-ecs-rng`
- `cargo check --tests -p kairo-ecs-core -p kairo-ecs-state -p kairo-ecs-types -p kairo-ecs-rng`
- `cargo clippy -p kairo-ecs-types -p kairo-ecs-core -p kairo-ecs-state -p kairo-ecs-rng --all-targets -- -D warnings`
- `cargo fmt --package kairo-ecs-core --package kairo-ecs-state --check`

## Known blockers

- Track 01E must remain a Rust facade-readiness lane until the Track 02 FFI contract and Track 12 fixture runner are accepted.
- Any future performance gate should be added as a real runnable benchmark check rather than a placeholder.

## Integration notes

- Keep the docs in sync if lane ownership or owned paths change.
- Keep test coverage aligned with the current deterministic scheduler, state, and RNG behavior.
- Do not widen this track into binding-path work from `Track 02` or fixture-runner work from `Track 12`.

## Contracts consumed

No additional consumed contracts were recorded by this Conductor hygiene update.


## Contracts changed

No contract changes were recorded by this Conductor hygiene update.


## Tests added

No tests were added by this Conductor hygiene update.


## Known risks

No new risks were introduced by this Conductor hygiene update.


## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.
