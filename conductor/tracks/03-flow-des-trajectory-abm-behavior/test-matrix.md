# Test Matrix — 03 The Flow: DES Trajectory API & ABM Behavior API

## Required tests

- `cargo test -p kairo-ecs-core` to cover the scheduler and shared event model that DES and ABM both depend on.
- `cargo test -p kairo-ecs-state` to keep the state transition layer deterministic while the trajectory and behavior APIs are being defined.
- `cargo test -p kairo-ecs-des --test des_resource_queue_v1` for the named DES fixture gate.
- `cargo test -p kairo-ecs-abm --test abm_behavior_update_v1` for the named ABM fixture gate.
- `cargo fmt --all --check` before any handoff that touches Rust code.
- `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo` to keep the conductor setup consistent.
- `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo` to prove the track is still accounted for in the wave policy and registry.
- `cargo test --workspace` once the track starts adding concrete DES and ABM code paths.
- `cargo test -p kairo-ecs-des -p kairo-ecs-abm` for the Track 03 deterministic trajectory and behavior-update smoke fixtures.

## Current CI commands

```bash
cargo fmt --all --check
cargo test -p kairo-ecs-core
cargo test -p kairo-ecs-state
cargo test -p kairo-ecs-des -p kairo-ecs-abm
pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo
pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo
```

## 2026-05-08 validation notes

- Passed: `cargo +stable-x86_64-pc-windows-gnu fmt --check -p kairo-ecs-des -p kairo-ecs-abm`.
- Passed: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des --test des_resource_queue_v1`.
- Passed: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-abm --test abm_behavior_update_v1`.
- Passed: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des -p kairo-ecs-abm`.
- Passed: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-core`.
- Passed: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-state`.
- Passed: `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo`.
- Passed: `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo`.
- Not rerun for this closeout: `cargo fmt --all --check`, because the working tree already contains unrelated local Conductor closeout edits. Focused Track 03 formatting passed.

## 2026-05-08 fixture-hardening validation

- Passed: `cargo +stable-x86_64-pc-windows-gnu fmt -p kairo-ecs-abm -p kairo-ecs-des --check`.
- Passed: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des -p kairo-ecs-abm`, including `des_resource_queue_v1` and `abm_behavior_update_v1`.
- Passed: `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`.
- Not claimed: shared fixture files under `conformance/fixtures/`; those remain follow-up work with Track 12 alignment.
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
