# 13 CI/CD, Code Quality & Supply Chain — plan.md

## Phase 0 — Track startup

- Read `conductor/workflow.md`.
- Read relevant contracts under `conductor/contracts/`.
- Confirm owned paths: `github workflows, deny.toml, codeql, dependabot/renovate`.
- Create `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md`.

## Phase 1 — Contract alignment

- Identify all public types, functions, schemas, commands, or package metadata this track consumes.
- Propose contract changes through ADR if required.
- Add workflow and supply-chain checks that point at the current workspace surfaces.

## Phase 2 — Scaffold

- Create package/crate/module skeleton.
- Add workflow smoke tests that prove the GitHub Actions gate is wired into CI.
- Add only concrete workflow notes that describe the current GitHub Actions jobs and record future follow-ups in the conductor docs.

## Phase 3 — Implementation

- Implement the smallest useful vertical slice.
- Add unit tests and integration tests.
- Add release and dependency policy checks only where the workflow consumes them.
- Add job-duration checks only if a workflow becomes a bottleneck.

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

