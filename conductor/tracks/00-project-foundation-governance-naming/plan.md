# 00 Project Foundation, Governance & Naming — plan.md

## Phase 0 — Track startup

- Read `conductor/workflow.md`.
- Read `conductor/product.md`, `conductor/tech-stack.md`, `conductor/workflow.md`, and the repo identity, naming, and release contracts under `conductor/contracts/`.
- Confirm owned paths: `README.md`, `LICENSE` or `LICENSE.md`, `governance/`, `docs/adr/`, `conductor/status.md`, and the naming checklist docs.
- Confirm the current control artifacts that already exist: `scripts/validate_conductor_setup.ps1`, `scripts/validate_track_coverage.ps1`, and the GitHub/registry setup docs this track governs.
- Refresh `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md` to match the active repo state.

## Phase 1 — Contract alignment

- Identify all public types, functions, schemas, commands, or package metadata this track consumes.
- Propose contract changes through ADR if required.
- Tie the foundation docs to the active control files and naming rules.

## Phase 2 — Scaffold

- Create package/crate/module skeleton.
- Align the root metadata, GitHub scaffolding, and naming rules with the concrete files already in the repo.
- Add validator coverage that proves the foundation gates run against the current repo.
- Align the foundation docs with the concrete control files and naming rules that govern the repo.

## Phase 3 — Implementation

- Implement the smallest useful vertical slice.
- Add unit tests and integration tests.
- Keep the foundation controls aligned with the shared conformance and track registry checks.
- Add timing checks only if the validation path itself becomes a measurable bottleneck.

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

