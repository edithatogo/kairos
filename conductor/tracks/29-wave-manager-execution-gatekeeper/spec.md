# Track 29: Wave Manager & Execution Gatekeeper

## Purpose

Enforce the wave policy (waves 0-5) — ensure Wave 0 tracks complete before Wave 1 starts, validate that no track advances without its dependencies, and own the critical-path gate.

## Why this track exists

KairoECS has 30+ tracks with complex inter-track dependencies. Without an automated wave gatekeeper, a subagent could merge a PR that depends on an incomplete dependency, corrupting the integration surface. This track owns the policy enforcement layer that prevents that.

## Primary subagent

`wave-manager-agent`

## Parallelization model

This track starts immediately and runs alongside all tracks. It does not produce runtime code. It owns policy documents, gate definitions, and the enforcement logic that blocks PR merges violating wave order.

## Inputs

- `conductor/tracks.yaml` — the canonical track inventory with dependencies.
- `conductor/wave-policy.md` — wave definitions and ordering rules.
- `conductor/track-map.md` — the expanded dependency DAG.
- `conductor/quality-gates.md` — existing gate definitions and conventions.

## Outputs

- A validated wave policy (`conductor/wave-policy.md`) with explicit wave membership, gating rules, and escalation paths.
- Gate definitions for `wave-progression-check` and `dependency-closure-check`.
- Enforcement logic that blocks a track from entering "In Progress" unless all dependencies are "Done".
- A critical-path heatmap showing which tracks gate other tracks.
- Handoff notes for release and CI subagents.

## Owned paths

- `conductor/wave-policy.md`
- `conductor/tracks/29-wave-manager-execution-gatekeeper/`
- `conductor/gates/wave-progression-check.yml`
- `conductor/gates/dependency-closure-check.yml`

## Blocked paths

- Implementation code in `crates/` — owned by Tracks 01-05.
- Binding source files in `bindings/` — owned by Tracks 06-11.
- CI/CD workflow files in `.github/workflows/` — owned by Track 13; this track defines gate logic only, not CI wiring.
- Release packaging manifests — owned by Track 15.

## Acceptance criteria

- Every track in `conductor/tracks.yaml` is assigned to exactly one wave (0-5).
- The wave policy prevents a Wave-N track from advancing to "In Progress" while any Wave-(N-1) dependency is not "Done".
- The `wave-progression-check` gate returns a blocking failure with a specific track ID and missing dependency when violated.
- The `dependency-closure-check` gate validates transitive dependency completion.
- The critical-path heatmap is updated on every track status change.
- A maintainer can override a wave gate only via a documented exception with an ADR.

## Release implications

- This track is **release-critical**. Any PR that violates wave order is blocked at merge.
- A gate failure in `wave-progression-check` or `dependency-closure-check` prevents release packaging.
- The wave policy becomes part of the release governance contract in Track 16.

## Non-goals

- Replacing the Conductor track lifecycle (Planned → In Progress → Done).
- Implementing CI/CD pipeline logic (owned by Track 13).
- Assigning or reassigning track ownership.
- Defining what "Done" means per track (each track owns its completion criteria).
