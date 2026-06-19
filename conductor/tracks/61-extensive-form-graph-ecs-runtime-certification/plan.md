# Track 61 Plan

## Phase 0: Sequential Components

- [ ] Task 0.1: Add failing tests for sequential node, information set, action edge, chance, and terminal utility components. Commit after passing as `track 61 task 0.1: add extensive form components`.
- [ ] Task 0.2: Add malformed tree and cycle-detection tests. Commit as `track 61 task 0.2: validate extensive form topology`.

Phase closeout: review, push, and GitHub Actions review.

## Phase 1: Traversal and Solvers

- [ ] Task 1.1: Add traversal tests using `ChildOf` and `TransitionTo` relations. Commit as `track 61 task 1.1: traverse extensive form graph ecs`.
- [ ] Task 1.2: Add backward-induction tests and implementation. Commit as `track 61 task 1.2: implement backward induction solver`.
- [ ] Task 1.3: Add imperfect-information fixture tests. Commit as `track 61 task 1.3: support information set fixtures`.

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

## Phase 2: Certification

- [ ] Task 2.1: Add end-to-end multi-game scenario manifest spanning normal-form and extensive-form games. Commit as `track 61 task 2.1: add multigame certification scenarios`.
- [ ] Task 2.2: Add certification validator and negative fixtures. Commit as `track 61 task 2.2: validate multigame certification evidence`.
- [ ] Task 2.3: Record benchmark, review, push, and GitHub Actions evidence. Commit as `track 61 task 2.3: record multigame certification closeout`.

Phase closeout repeats review, push, and GitHub Actions review.
