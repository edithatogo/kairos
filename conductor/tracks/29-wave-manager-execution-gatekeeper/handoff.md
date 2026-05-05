# Handoff: Track 29 Wave Manager & Execution Gatekeeper

## Summary

Defined the wave policy enforcement layer for the KairoECS track system. All tracks are assigned to waves 0-5 derived from the dependency graph. Two gates — `wave-progression-check` and `dependency-closure-check` — block track advancement and release packaging when dependencies are unsatisfied.

## Files changed

`conductor/tracks/29-wave-manager-execution-gatekeeper/plan.md`, `conductor/tracks/29-wave-manager-execution-gatekeeper/spec.md`, `conductor/tracks/29-wave-manager-execution-gatekeeper/test-matrix.md`, `conductor/tracks/29-wave-manager-execution-gatekeeper/handoff.md`, `conductor/wave-policy.md`, `conductor/gates/wave-progression-check.yml`, `conductor/gates/dependency-closure-check.yml`, `conductor/quality-gates.md`, `conductor/track-map.md`

## Contracts consumed

- `conductor/tracks.yaml` — canonical track inventory and dependency source.
- `conductor/track-map.md` — expanded dependency DAG.
- `conductor/quality-gates.md` — gate definition conventions.

## Release gates affected

- **wave-progression-check**: Blocks any PR that moves a track to "In Progress" when its `depends_on` are not all "Done".
- **dependency-closure-check**: Blocks release packaging if any transitive dependency is incomplete.
- Both gates are wired into the release-critical path and referenced from `conductor/quality-gates.md`.

## Risks and unresolved questions

- The wave policy may need adjustment if track dependencies change. Each change must update `conductor/wave-policy.md`.
- Transitive dependency closure for 35+ tracks is computationally trivial but must be kept synchronized with `conductor/tracks.yaml`.
- Maintainer overrides must be tracked to prevent erosion of the wave policy. Each override requires an ADR.
- Tracks that are parallel-safe but have wave constraints may create confusion — the wave policy clarifies that parallel-safe means "starts immediately" but does not override dependency gating.
