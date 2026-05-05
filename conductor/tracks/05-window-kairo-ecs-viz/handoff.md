# Handoff — 05 The Window: kairo-ecs-viz Visualization

## Summary

Track 05 is still in docs-and-visualization planning mode. The repo currently has the website build/dev scripts and `website/src/index.md`, but no `kairo-ecs-viz` package or visualization example package yet. This track is defining the rendering contract against the current state and scheduler surfaces before the package exists.

## Files changed

No code files were changed in this handoff pass.

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/core-contract.md`, and `conductor/contracts/conformance-contract.md`.

## Contracts changed

No contracts have been changed yet for this track.

## Tests added

No track-specific tests were added yet. The active gate is the shared Rust test surface, the website build, and the conductor validators in `scripts\validate_conductor_setup.ps1` and `scripts\validate_track_coverage.ps1`.

## Known risks

The visualization contract needs to stay aligned with the core state and scheduler surfaces so the docs and examples do not drift from the implementation. A mismatch here would make the website examples misleading even if the build still passes.

## Integration notes

Next step: publish the first visualization input model and wire it to the existing website build path before introducing a package crate or any render-specific example code.
