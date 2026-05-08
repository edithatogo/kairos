# Handoff — 03 The Flow: DES Trajectory API & ABM Behavior API

## Summary

Track 03 now has a minimal R2 implementation slice for the DES trajectory API and ABM behavior API. The DES crate wraps the shared deterministic scheduler with a fixed-tick `TrajectoryRequest`/`Trajectory` request and trace surface. The ABM crate keeps the lightweight `ABMContext` component/scheduler facade and adds a deterministic `BehaviorSimulation`, `AgentBehavior`, `BehaviorContext`, and behavior-decision loop backed by per-agent RNG streams.

## Files changed

- `Cargo.toml`
- `Cargo.lock`
- `crates/kairo-ecs-des/Cargo.toml`
- `crates/kairo-ecs-des/src/lib.rs`
- `crates/kairo-ecs-des/tests/des_integration.rs`
- `crates/kairo-ecs-des/tests/des_resource_queue_v1.rs`
- `crates/kairo-ecs-abm/Cargo.toml`
- `crates/kairo-ecs-abm/src/lib.rs`
- `crates/kairo-ecs-abm/tests/abm_integration.rs`
- `crates/kairo-ecs-abm/tests/abm_behavior_update_v1.rs`
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
- `crates/kairo-ecs-des/tests/des_resource_queue_v1.rs`: named DES resource queue fixture covering FIFO admission and fixed-tick trajectory replay ordering.
- `crates/kairo-ecs-abm/src/lib.rs`: scheduler-ordered behavior updates, event-budget behavior, deterministic entity-RNG replay, and despawn decisions.
- `crates/kairo-ecs-abm/tests/abm_integration.rs`: component attachment and multi-agent scheduling smoke coverage.
- `crates/kairo-ecs-abm/tests/abm_behavior_update_v1.rs`: named ABM behavior-update fixture covering scheduler order and deterministic per-agent RNG replay.

## Validation run

- `cargo +stable-x86_64-pc-windows-gnu fmt --check -p kairo-ecs-abm -p kairo-ecs-des` passed on 2026-05-08.
- `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des --test des_resource_queue_v1` passed on 2026-05-08.
- `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-abm --test abm_behavior_update_v1` passed on 2026-05-08.
- `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des -p kairo-ecs-abm` passed on 2026-05-08.
- `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-core` passed on 2026-05-08.
- `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-state` passed on 2026-05-08.
- `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo` passed on 2026-05-08.
- `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo` passed on 2026-05-08.
- `cargo fmt --all --check` was not rerun for this closeout because the working tree already contains unrelated local Conductor closeout edits; focused Track 03 formatting passed.

## 2026-05-08 fixture hardening

- Added `des_resource_queue_v1` and `abm_behavior_update_v1` named integration fixtures under Track 03-owned crate test paths.
- `cargo +stable-x86_64-pc-windows-gnu fmt --check -p kairo-ecs-abm -p kairo-ecs-des` passed.
- `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des -p kairo-ecs-abm` passed with 22 tests across ABM and DES unit, integration, and named fixture tests.
- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` passed.

## Known risks

The current APIs are intentionally minimal. They do not yet export shared conformance fixture files under `conformance/fixtures/des_resource_queue_v1`, `abm_behavior_update_v1`, or `hybrid_des_abm_v1`. The ABM update kind is currently a crate-local `EventKind::Custom` value and should be reconciled if Track 01 later introduces domain-specific event kinds.

## Integration notes

Next step: bind the DES and ABM smoke paths to shared conformance fixture files under `conformance/fixtures/`, then add richer resource/queue and agent-decision examples under `examples/flow/`.

## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.
## Phase closeout evidence

2026-05-08 fixture-hardening closeout:

- `$conductor-review` result: the untracked DES/ABM fixture tests are in Track 03-owned paths and should be retained as implementation-slice hardening evidence.
- Accepted fixes: added the two named fixture tests and linked them to Track 03 handoff, test matrix, status, and phase-closeout evidence.
- Deferred or blocked fixes: shared fixture files under `conformance/fixtures/des_resource_queue_v1`, `conformance/fixtures/abm_behavior_update_v1`, and `conformance/fixtures/hybrid_des_abm_v1` remain future Track 03/12 integration work.
- Validation commands: `cargo +stable-x86_64-pc-windows-gnu fmt -p kairo-ecs-abm -p kairo-ecs-des --check`, `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des -p kairo-ecs-abm`, and `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`.
- Cleanup state: the only in-scope dirty files were the two new Track 03 tests and their Conductor evidence updates.
- Commit SHA / pushed ref: `5dd1937566898b2e028ac61dab1e9dd173e6d919` on `origin/main` is the current pushed base for this local closeout pass.
- Strict cleanup gate: run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after the commit and push.
- Next-phase decision: Track 03 is `In Review`; reviewer signoff is still required before moving the DES/ABM flow APIs to `Done`.

2026-05-08 review closeout:

- `$conductor-review` result: one documentation evidence gap was found in `examples/flow/README.md`; no DES or ABM behavioral findings were found.
- Accepted fixes: added the flow example maturity label, reproducibility commands, and expected output for the named DES/ABM fixture gates.
- Deferred or blocked fixes: shared JSON fixture exports and richer hybrid/model-zoo scenarios remain Track 12/23 follow-up work.
- Validation commands: `cargo +stable-x86_64-pc-windows-gnu fmt --check -p kairo-ecs-des -p kairo-ecs-abm`, `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des --test des_resource_queue_v1`, `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-abm --test abm_behavior_update_v1`, `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des -p kairo-ecs-abm`, `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-core`, `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-state`, `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo`, `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo`, and `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`.
- Cleanup state: strict clean-tree closeout was not run because the shared worktree already contains unrelated staged and unstaged edits across other tracks.
- Commit SHA / pushed ref: `ee8c123e0a6dddd27986e7e657642190ee4f2560` on `origin/main` is the current base for this local review closeout pass.
- Next-phase decision: Track 03 is `Done`; future shared conformance fixture exports, hybrid scenarios, and model-zoo examples should be handled by Track 12/23 follow-up work.

2026-05-08 ABM despawn regression fix:

- `$conductor-review` result: one high-severity behavioral bug was found in `BehaviorSimulation::run_for`; a future queued update could still invoke behavior for an agent already despawned by an earlier update.
- Accepted fixes: skip non-live agents before resolving per-agent RNG streams and invoking behavior callbacks, and add a regression test for despawned-agent future events.
- Validation commands: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-abm --test abm_behavior_update_v1` and `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-abm`.
- Next-phase decision: Track 03 remains `Done`.
