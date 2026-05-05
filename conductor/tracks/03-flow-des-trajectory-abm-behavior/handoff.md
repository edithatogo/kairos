# Handoff — 03 The Flow: DES Trajectory API & ABM Behavior API

## Summary

Track 03 is still defining the split between the DES trajectory API and the ABM behavior API. The repo already has the shared core surfaces in `crates/kairo-ecs-core`, `crates/kairo-ecs-types`, `crates/kairo-ecs-state`, and `crates/kairo-ecs-rng`, plus the example README stubs under `examples/des/` and `examples/abm/` that point at the future package boundary.

## Files changed

No code files were changed in this handoff pass.

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/core-contract.md`, and `conductor/contracts/conformance-contract.md`.

## Contracts changed

No contracts have been changed yet for this track.

## Tests added

No track-specific tests were added yet. The active gate is the shared Rust test surface plus the conductor validators in `scripts\validate_conductor_setup.ps1` and `scripts\validate_track_coverage.ps1`.

## Known risks

The DES trajectory API and the ABM behavior API still need a final ownership split before implementation can start without overlap. Both surfaces also need to stay aligned with the shared scheduler and state semantics so that deterministic ordering does not drift.

## Integration notes

Next step: land the DES and ABM package skeletons, then bind deterministic event-ordering and behavior-update fixtures to the shared core crates before widening the public API.
