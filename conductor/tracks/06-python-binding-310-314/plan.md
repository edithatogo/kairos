# 06 Python Binding 3.10-3.14 — plan.md

## Phase 0 — Track startup

- Read `conductor/workflow.md`.
- Read relevant contracts under `conductor/contracts/`.
- Confirm owned paths for this review pass: `bindings/python`.
- Create `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md`.

## Phase 1 — Contract alignment

- Identify all public types, functions, schemas, commands, or package metadata this track consumes.
- Propose contract changes through ADR if required.
- Add Python fixture references tied to the supported interpreter range.

## Phase 2 — Scaffold

- Maintain the package skeleton already present under `bindings/python/kairo_ecs`.
- Add import and wheel smoke tests for the supported Python matrix.
- Document the binding package boundary, supported interpreter range, and fixture hookup in the package README once the package boundary is in place.

## Phase 3 — Implementation

- Implement the smallest useful vertical slice.
- Add unit tests and integration tests.
- Add fixture bridge checks for the Python binding surface.
- Add wheel build or import timing checks only if the binding surface needs them.

## Phase 4 — Cross-track integration

- Run owned tests.
- Run affected shared conformance tests.
- Update docs and release notes.
- Ensure no other subagent-owned paths were modified without handoff.

## Phase 5 — Closeout

- Complete `handoff.md`.
- Record risks and follow-up tasks.
- Confirm CI gates.
- Mark track ready for integration.
## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` plus the gates listed in `test-matrix.md`.
5. Commit and push the cleaned slice, then record the commit SHA or blocker in `handoff.md`.
6. Advance the next phase only after there is no in-scope unstaged or untracked work except documented draft satellites.