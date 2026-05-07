# 05 The Window: kairo-ecs-viz Visualization — plan.md

## Phase 0 — Track startup

- Read `conductor/workflow.md`, `conductor/contracts/core-contract.md`, and `conductor/contracts/conformance-contract.md`.
- Read the current `website` scaffold in `website/package.json`, `website/scripts/build.js`, `website/scripts/dev.js`, and `website/src/index.md`.
- Confirm the intended owned paths remain `crates/kairo-ecs-viz`, `examples/viz`, and `website/docs/visualization/`; the headless viz crate, example package, and visualization docs now exist as a minimal implementation slice.
- Keep `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md` current as the track evolves.

## Phase 1 — Contract alignment

- Define the visualization contract for state snapshots, frame updates, and example inputs.
- Map the data this track consumes from the core state and scheduler surfaces.
- Propose contract changes through ADR if required.
- Add fixture stubs for the rendering inputs that should stay stable across docs and examples.

## Phase 2 — Scaffold

- Maintain the `kairo-ecs-viz` crate skeleton already registered in the workspace.
- Add visualization smoke tests that prove the shared workspace checks are wired into CI.
- Use the existing website build and dev scripts to host the visualization docs while renderer-backed examples remain future work.

## Phase 3 — Implementation

- Implement the smallest useful vertical slice: a single rendered state snapshot or event-stream frame.
- Add deterministic conversion from the Track 01 `WorldSnapshot` surface into a headless `RenderFrame`.
- Add unit tests and integration tests.
- Add fixture-driven checks only for the visualization inputs this track owns.
- Add benchmarks where frame generation or rendering becomes performance-sensitive.

## Phase 4 — Cross-track integration

- Run owned tests.
- Run affected shared conformance tests.
- Update docs and release notes.
- Ensure no other subagent-owned paths were modified without handoff, especially tracks 01, 04, and 12.

## Phase 5 — Closeout

- Complete `handoff.md`.
- Record the remaining rendering and documentation tasks.
- Confirm CI gates.
- Mark the track ready for the next implementation wave, not as finished.
## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` plus the gates listed in `test-matrix.md`.
5. Commit and push the cleaned slice, then record the commit SHA or blocker in `handoff.md`.
6. Advance the next phase only after there is no in-scope unstaged or untracked work except documented draft satellites.