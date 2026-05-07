# Test Matrix — 01 The Heart: kairo-ecs-core & kairo-ecs-state

## Local gates

- `cargo fmt --all --check`
- `cargo clippy -p kairo-ecs-types -p kairo-ecs-core -p kairo-ecs-state -p kairo-ecs-rng --all-targets -- -D warnings`
- `cargo check --tests -p kairo-ecs-types -p kairo-ecs-core -p kairo-ecs-state -p kairo-ecs-rng`
- `cargo check --tests -p kairo-ecs-state`
- `cargo test -p kairo-ecs-types`
- `cargo test -p kairo-ecs-core`
- `cargo test -p kairo-ecs-state`
- `cargo test -p kairo-ecs-rng`
- `pwsh -NoProfile -Command "Test-Path 'conformance/fixtures/deterministic_ordering.json'"`
- `pwsh -NoProfile -Command "Test-Path 'conformance/fixtures/cancellation.json'"`
- `pwsh -NoProfile -Command "Test-Path 'conformance/fixtures/rng_replay.json'"`
- `pwsh -NoProfile -File scripts/validate_conductor_setup.ps1 -SkipCargo`
- `pwsh -NoProfile -File scripts/validate_track_coverage.ps1 -SkipCargo`

## Full-link gate

- `cargo test --workspace`

## Current validation note

Run the full `cargo test -p kairo-ecs-types -p kairo-ecs-core -p kairo-ecs-state -p kairo-ecs-rng` gate when a working MSVC linker environment is active. Keep the `cargo check --tests` gates in the matrix as the linker-safe local fallback for shells where executable test linking is not available.

## Review-hardening coverage

The Track 01 scheduler tests must keep coverage for:

- ordering by time, priority, and insertion sequence
- cancellation of unknown IDs
- cancellation of already-dispatched IDs
- duplicate cancellation
- cancelled future events that should not force a false `LimitReached`
- scheduler stats for scheduled, dispatched, cancelled, pending, and current simulation time
- pure Rust facade status mapping for dispatch, empty, limit, invalid priority, and not-found cancellation

The Track 01 state tests must keep coverage for:

- deterministic snapshot ordering for live entities
- same-entity component replacement without duplicate dense rows
- stale generations failing to read or remove the current component
- newer generations superseding stale indexed rows deterministically

## Lane checks

- 01A Types and time: `cargo test -p kairo-ecs-types`
- 01B Scheduler: `cargo test -p kairo-ecs-core`
- 01C State: `cargo test -p kairo-ecs-state`
- 01D RNG: `cargo test -p kairo-ecs-rng`
- 01E Facade readiness: `cargo test -p kairo-ecs-core`
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.