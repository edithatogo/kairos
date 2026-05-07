# Handoff — 03 The Flow: DES Trajectory API & ABM Behavior API

## Summary

Track 03 now has a minimal R2 implementation slice for the DES trajectory API and ABM behavior API. The DES crate wraps the shared deterministic scheduler with a fixed-tick `TrajectoryRequest`/`Trajectory` request and trace surface. The ABM crate keeps the lightweight `ABMContext` component/scheduler facade and adds a deterministic `BehaviorSimulation`, `AgentBehavior`, `BehaviorContext`, and behavior-decision loop backed by per-agent RNG streams.

## Files changed

- `Cargo.toml`
- `Cargo.lock`
- `crates/kairo-ecs-des/Cargo.toml`
- `crates/kairo-ecs-des/src/lib.rs`
- `crates/kairo-ecs-des/tests/des_integration.rs`
- `crates/kairo-ecs-abm/Cargo.toml`
- `crates/kairo-ecs-abm/src/lib.rs`
- `crates/kairo-ecs-abm/tests/abm_integration.rs`
- `examples/flow/README.md`
- `conductor/tracks/03-flow-des-trajectory-abm-behavior/test-matrix.md`
- `conductor/tracks/03-flow-des-trajectory-abm-behavior/handoff.md`

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/core-contract.md`, and `conductor/contracts/conformance-contract.md`. The new APIs preserve fixed-tick time, scheduler ordering by `(time_ticks ASC, priority ASC, sequence ASC)`, generational entity handles, and deterministic fixture coverage for public behavior.

## Contracts changed

No shared contracts were changed for this track.

## Tests added

- `crates/kairo-ecs-des/src/lib.rs`: scheduler-order replay and bounded trajectory smoke tests.
- `crates/kairo-ecs-des/tests/des_integration.rs`: FIFO resource queue and fixed-tick scheduling smoke coverage.
- `crates/kairo-ecs-abm/src/lib.rs`: scheduler-ordered behavior updates, event-budget behavior, deterministic entity-RNG replay, and despawn decisions.
- `crates/kairo-ecs-abm/tests/abm_integration.rs`: component attachment and multi-agent scheduling smoke coverage.

## Validation run

- `cargo fmt -p kairo-ecs-abm -p kairo-ecs-des --check` passed on 2026-05-07.
- `cargo test -p kairo-ecs-des -p kairo-ecs-abm` passed on 2026-05-07.
- `cargo test -p kairo-ecs-core` passed on 2026-05-07.
- `cargo test -p kairo-ecs-state` passed on 2026-05-07.
- `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo` passed on 2026-05-07.
- `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo` passed on 2026-05-07.
- `cargo fmt --all --check` is still blocked by unrelated in-flight formatting diffs outside Track 03, including `crates/kairo-ecs-arrow`, `crates/kairo-ecs-debug`, `crates/kairo-ecs-rng`, `crates/kairo-ecs-types`, and `crates/kairo-ecs-wasm`.

## Known risks

The current APIs are intentionally minimal. They do not yet export shared conformance fixture files under `conformance/fixtures/des_resource_queue_v1`, `abm_behavior_update_v1`, or `hybrid_des_abm_v1`. The ABM update kind is currently a crate-local `EventKind::Custom` value and should be reconciled if Track 01 later introduces domain-specific event kinds.

## Integration notes

Next step: bind the DES and ABM smoke paths to shared conformance fixtures, then add richer resource/queue and agent-decision examples under `examples/flow/`.

## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.
