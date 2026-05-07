# 03 The Flow: DES Trajectory API & ABM Behavior API — plan.md

## Phase 0 — Track startup

- Read `conductor/workflow.md`, `conductor/contracts/core-contract.md`, and `conductor/contracts/conformance-contract.md`.
- Read the current shared scaffolds in `crates/kairo-ecs-core`, `crates/kairo-ecs-types`, `crates/kairo-ecs-state`, `crates/kairo-ecs-rng`, and the example README stubs under `examples/des/` and `examples/abm/`.
- Confirm the intended owned paths remain `crates/kairo-ecs-des`, `crates/kairo-ecs-abm`, and `examples/flow/`; the crate directories now exist and this track is in a minimal implementation-slice state.
- Create `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md`.

## Phase 1 — Contract alignment

- Define the split between the DES trajectory API and the ABM behavior API.
- Map the shared scheduler, time, and state semantics that both surfaces must preserve.
- Propose contract changes through ADR if required.
- Add fixture stubs for the event ordering and behavior-update paths that will later consume shared conformance fixtures.

## Phase 2 — Scaffold

- Maintain the DES and ABM crate skeletons already registered in the workspace.
- Keep DES/ABM smoke tests proving the shared scheduler ordering and deterministic behavior-update paths compile under package-focused gates.
- Add docs stubs that name the concrete DES trajectory and ABM behavior follow-ups without claiming the packages are complete.

## Phase 3 — Implementation

- Implement the smallest useful vertical slice: a deterministic trajectory request/response path for DES and a single behavior-update loop for ABM.
- Add unit tests and integration tests.
- Add fixture parity checks where the flow API consumes Track 12 outputs.
- Add benchmarks where queue churn or event scheduling becomes performance-sensitive.

## Phase 4 — Cross-track integration

- Run owned tests.
- Run affected shared conformance tests.
- Update docs and release notes.
- Ensure no other subagent-owned paths were modified without handoff, especially tracks 01, 04, 06-11, and 12.

## Phase 5 — Closeout

- Complete `handoff.md`.
- Record the remaining API decisions and follow-up tasks.
- Confirm CI gates.
- Mark the track ready for the next implementation wave, not as finished.
## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Update `conductor/phase-closeout.yaml` with review outcome, accepted fixes, validation commands, cleanup state, commit SHA or blocker, pushed ref, and next-phase decision.
5. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` plus the gates listed in `test-matrix.md`.
6. Commit and push the cleaned slice, then record the commit SHA or blocker in `handoff.md`.
7. Run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` to verify recorded commits, pushed refs, and cleanup state.
8. Advance the next phase only after there is no in-scope unstaged or untracked work except documented draft satellites.