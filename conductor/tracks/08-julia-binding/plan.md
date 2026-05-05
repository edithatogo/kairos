# 08 Julia Binding — plan.md

## Phase 0 — Track startup

- Read `conductor/workflow.md`.
- Read relevant contracts under `conductor/contracts/`.
- Confirm owned paths: `bindings/julia`.
- Keep packaging, registry, and release dry-run work out of this slice.
- Create `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md`.

## Phase 1 — Contract alignment

- Identify all public types, functions, schemas, commands, or package metadata this track consumes.
- Propose contract changes through ADR if required.
- Add Julia fixture references tied to the package boundary and project setup.

## Phase 2 — Scaffold

- Create package/crate/module skeleton.
- Add package smoke tests for the Julia binding surface.
- Document the package boundary, supported Julia project setup, and fixture bridge in the package README once the package boundary is in place.

## Phase 3 — Implementation

- Implement the smallest useful vertical slice.
- Add unit tests and integration tests.
- Add fixture bridge checks for exported Julia helpers.
- Add project load timing checks only if the Julia surface needs them.

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

