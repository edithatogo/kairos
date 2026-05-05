# 16 Release Governance & Maintenance — plan.md

## Phase 0 — Track startup

- Read `conductor/workflow.md`.
- Read relevant contracts under `conductor/contracts/`.
- Confirm owned paths: `CHANGELOG, docs/release, governance docs`.
- Create `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md`.

## Phase 1 — Contract alignment

- Identify all public types, functions, schemas, commands, or package metadata this track consumes.
- Propose contract changes through ADR if required.
- Add release checklist references tied to the current package and governance surfaces.

## Phase 2 — Scaffold

- Create package/crate/module skeleton.
- Add release smoke checks that prove the governance docs are wired into CI.
- Add release notes and governance docs that describe the real release path, with follow-ups recorded in conductor trackers.

## Phase 3 — Implementation

- Implement the smallest useful vertical slice.
- Add unit tests and integration tests.
- Add release-readiness checks only where fixture evidence blocks a release.
- Add release-process timing checks only if they reveal a bottleneck.

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

