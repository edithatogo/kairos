# 14 Documentation Site & Education — plan.md

## Phase 0 — Track startup

- Read `conductor/workflow.md`.
- Read relevant contracts under `conductor/contracts/`.
- Confirm owned paths: `docs/`, `website/`, `examples/docs/`, and `templates/website/`.
- Create `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md`.

## Phase 1 — Contract alignment

- Identify all public types, functions, schemas, commands, or package metadata this track consumes.
- Propose contract changes through ADR if required.
- Add fixture examples for the docs home, nav links, and any rendered markdown fragment that the site consumes.

## Phase 2 — Scaffold

- Create the docs site skeleton under `website/`.
- Keep the site buildable with `npm ci` and `npm run build`.
- Add smoke tests that run `npm ci`, `npm run build`, and verify `website/build/index.html` plus the docs-tree link check.
- Add draft docs pages for the current repository docs tree, then replace any unresolved task markers with tracked follow-up issues.

## Phase 3 — Implementation

- Implement the smallest useful vertical slice.
- Point the docs home at the current repo docs tree.
- Add unit tests and integration tests where the site renderer needs them.
- Add docs-link and site-content checks for the current website surface.
- Add render or build timing checks only for the docs site path.

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

