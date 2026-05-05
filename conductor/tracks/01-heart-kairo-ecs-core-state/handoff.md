# Handoff — 01 The Heart: kairo-ecs-core & kairo-ecs-state

## Summary

Track 01 is anchored by `lanes.md`, the shared conformance fixtures, and the workspace validators. The current control surface is `conformance/fixtures/manifest.json`, `conformance/fixtures/deterministic_ordering.json`, `conformance/fixtures/cancellation.json`, `conformance/fixtures/rng_replay.json`, `scripts/validate_conductor_setup.ps1`, and `scripts/validate_track_coverage.ps1`.

## Files changed

No code files were changed in this handoff pass.

## Contracts consumed

The track consumes the scheduler, state, types, and RNG contracts that are staged through `lanes.md` and the conformance fixture set.

## Contracts changed

The next contract surface is the deterministic ordering and replay behavior in `crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, and `crates/kairo-ecs-rng`.

## Tests added

The track now has a concrete validator path through `scripts/validate_conductor_setup.ps1` and `scripts/validate_track_coverage.ps1`, plus fixture-presence checks for the three bootstrap conformance files.

## Known risks

The main risk is drifting out of sync with `lanes.md` if the implementation slices change without updating the fixture and validation docs.

## Integration notes

Next implementation step: land the first vertical slice for time, identity, and deterministic event ordering, then keep the fixture manifest and validator coverage aligned as each lane closes.
