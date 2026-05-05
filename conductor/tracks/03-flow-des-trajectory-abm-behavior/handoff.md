# Handoff — 03 The Flow: DES Trajectory API & ABM Behavior API

## Summary

Track 03 now has a minimal R2 implementation slice for the DES trajectory API and ABM behavior API. The DES crate wraps the shared deterministic scheduler with a fixed-tick `Trajectory` request/trace surface. The ABM crate wraps the shared scheduler, entity store, and deterministic RNG with a `BehaviorSimulation` and `AgentBehavior` update contract.

## Files changed

- `Cargo.toml`
- `Cargo.lock`
- `crates/kairo-ecs-des/Cargo.toml`
- `crates/kairo-ecs-des/src/lib.rs`
- `crates/kairo-ecs-abm/Cargo.toml`
- `crates/kairo-ecs-abm/src/lib.rs`
- `examples/flow/README.md`
- `conductor/tracks/03-flow-des-trajectory-abm-behavior/test-matrix.md`
- `conductor/tracks/03-flow-des-trajectory-abm-behavior/handoff.md`

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/core-contract.md`, and `conductor/contracts/conformance-contract.md`. The new APIs preserve fixed-tick time, scheduler ordering by `(time_ticks ASC, priority ASC, sequence ASC)`, generational entity handles, and deterministic fixture coverage for public behavior.

## Contracts changed

No shared contracts were changed for this track.

## Tests added

- `crates/kairo-ecs-des/src/lib.rs`: scheduler-order replay and bounded trajectory smoke tests.
- `crates/kairo-ecs-abm/src/lib.rs`: scheduler-ordered behavior updates and deterministic run-seed replay smoke tests.

## Validation run

- `cargo fmt -p kairo-ecs-abm -p kairo-ecs-des --check` passed.
- `cargo check -p kairo-ecs-des -p kairo-ecs-abm` passed.
- `cargo check --tests -p kairo-ecs-des -p kairo-ecs-abm` passed.
- `cargo test -p kairo-ecs-des -p kairo-ecs-abm` reached the linker step but did not run because this shell resolves `link.exe` to `C:\Users\60217257\scoop\apps\git\current\usr\bin\link.exe`, which fails before producing Windows test binaries.

## Known risks

The current APIs are intentionally minimal. They do not yet export shared conformance fixture files under `conformance/fixtures/des_resource_queue_v1`, `abm_behavior_update_v1`, or `hybrid_des_abm_v1`. The ABM update kind is currently a crate-local `EventKind::Custom` value and should be reconciled if Track 01 later introduces domain-specific event kinds.

## Integration notes

Next step: bind the DES and ABM smoke paths to shared conformance fixtures, then add richer resource/queue and agent-decision examples under `examples/flow/`.
