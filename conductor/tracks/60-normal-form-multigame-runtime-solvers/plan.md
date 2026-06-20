# Track 60 Plan

## Phase 0: Component Model

- [x] Task 0.1: Add failing tests for `PayoffMatrix`, `StrategySpace`, and `Utility` invariants. Commit `8fd3e87 track 60 task 0.1: add normal form components`.
- [x] Task 0.2: Add invalid-shape and invalid-utility negative tests. Commit `c97c7d7 track 60 task 0.2: validate normal form component invariants`.

Phase closeout: run validators, `$conductor-review 60`, apply fixes, update handoff, push, and review GitHub Actions.

## Phase 1: Solver Systems

- [x] Task 1.1: Add best-response solver tests and implementation. Commit `track 60 task 1.1: implement best response solver`.
- [ ] Task 1.2: Add pure Nash equilibrium fixture tests and implementation. Commit as `track 60 task 1.2: implement pure nash solver`.
- [ ] Task 1.3: Add dominated-strategy elimination tests and implementation. Commit as `track 60 task 1.3: implement dominated strategy elimination`.

Phase closeout repeats review, push, and GitHub Actions review.

## Phase closeout gate

Before any phase is accepted:

- Run `$conductor-review` for this track.
- Auto-apply accepted review fixes that are in scope for this track.
- Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`.
- Keep `conductor/tracks.yaml`, `conductor/tracks.md`, `conductor/phase-closeout.yaml`, and `conductor/status.md` synchronized.
- Commit and push the cleaned slice.
- Run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`.
- Run `gh pr checks --watch` and record the GitHub Actions review result.

## Phase 2: Performance and Evidence

- [ ] Task 2.1: Add flat-array benchmark fixtures and evidence manifest entries. Commit as `track 60 task 2.1: record normal form solver benchmarks`.
- [ ] Task 2.2: Add docs and examples for normal-form execution. Commit as `track 60 task 2.2: document normal form runtime`.

Phase closeout repeats review, push, and GitHub Actions review.
