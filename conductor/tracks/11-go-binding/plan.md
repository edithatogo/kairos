# 11 Go Binding — plan.md

## Phase 0 — Track startup

- Read `conductor/workflow.md`.
- Read relevant contracts under `conductor/contracts/`.
- Confirm owned paths: `bindings/go`.
- Keep module release, proxy, and tag preparation out of this slice.
- Create `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md`.

## Phase 1 — Contract alignment

- Identify all public types, functions, schemas, commands, or package metadata this track consumes.
- Propose contract changes through ADR if required.
- Add Go fixture references tied to the module boundary and toolchain range.

## Phase 2 — Scaffold

- Create package/crate/module skeleton.
- Add module smoke tests for the Go binding surface.
- Document the module boundary, supported Go toolchain, and fixture bridge in the module README once the module boundary is in place.

## Phase 3 — Implementation

- Implement the smallest useful vertical slice.
- Add unit tests and integration tests.
- Add fixture bridge checks for exported Go helpers.
- Add build or import timing checks only if the Go surface needs them.

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

