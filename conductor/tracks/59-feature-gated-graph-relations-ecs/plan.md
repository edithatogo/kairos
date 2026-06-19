# Track 59 Plan

## Phase 0: Feature Boundary

- [ ] Task 0.1: Add failing compile tests proving `graph-relations` APIs are unavailable by default. Commit after passing as `track 59 task 0.1: enforce graph feature boundary`.
- [ ] Task 0.2: Add the Cargo feature and module export behind `cfg(feature = "graph-relations")`. Commit as `track 59 task 0.2: add graph relations feature`.

Phase closeout: review, push, and GitHub Actions review.

## Phase 1: Components and Traversal

- [ ] Task 1.1: Add `ChildOf` and `TransitionTo` component tests. Commit as `track 59 task 1.1: add graph edge components`.
- [ ] Task 1.2: Add traversal over flat arrays using Entity IDs only. Commit as `track 59 task 1.2: traverse graph relations over ecs arrays`.
- [ ] Task 1.3: Add no raw pointer, no self-reference, and no `Box` topology scan. Commit as `track 59 task 1.3: enforce no pointer graph topology`.

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
