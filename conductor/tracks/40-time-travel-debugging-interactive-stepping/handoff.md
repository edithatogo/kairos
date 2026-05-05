# Handoff — 40 Time-Travel Debugging & Interactive Stepping

## Summary

Track 40 is in spec-design mode. No `kairo-ecs-debug` crate exists yet. This track defines the time-travel debugging contracts — event trace format, snapshot + delta encoding, debugger CLI, and browser timeline scrubber — before any implementation lands. The trace recorder observes the scheduler without modifying its behavior.

## Files changed

No code files were changed in this handoff pass.

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/conformance-contract.md`, Track 01 handoff notes (deterministic scheduler hooks), Track 12 handoff notes (conformance snapshot format), Track 22 experiment runner CLI interface.

## Contracts changed

No contracts have been changed yet for this track.

## Tests added

No track-specific tests were added yet. The active gate is the workspace compilation check plus the conductor validators in `scripts\validate_conductor_setup.ps1` and `scripts\validate_track_coverage.ps1`.

## Known risks

The trace file size for large simulations (10M+ events) is the primary scalability concern and will require delta encoding with sparse snapshots from day one. Forward/backward parity is the critical correctness property — a single divergence would undermine user trust in the debugger. The browser timeline scrubber must remain a non-core dependency so the workspace builds without it.

## Integration notes

Next step: publish the event trace format contract with the scheduler observer trait, then scaffold the debug crate once the core event dispatch hook points are defined and the conformance snapshot format is stable.
