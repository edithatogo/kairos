# Handoff — 04 The Analyst: kairo-ecs-arrow Telemetry

## Summary

Track 04 is still in schema-design mode. The repo currently has the `schemas/arrow` documentation surface and the shared conformance fixtures, but no `kairo-ecs-arrow` package yet. This track is defining the exporter contract against the core event model before any package implementation lands.

## Files changed

No code files were changed in this handoff pass.

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/arrow-schema-contract.md`, and `conductor/contracts/conformance-contract.md`.

## Contracts changed

No contracts have been changed yet for this track.

## Tests added

No track-specific tests were added yet. The active gate is the shared Rust test surface plus the conductor validators in `scripts\validate_conductor_setup.ps1` and `scripts\validate_track_coverage.ps1`.

## Known risks

The schema needs to stay aligned with the core event model and the conformance fixtures so that telemetry does not drift from the rest of the workspace. If the Arrow field names move ahead of the event contracts, downstream consumers will not be able to validate the emitted payloads cleanly.

## Integration notes

Next step: publish the first Arrow schema artifact, then wire a minimal telemetry export path to it once the package skeleton exists and the event tags are stable.
