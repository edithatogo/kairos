# Track 58 Plan

## Phase 0: Codegen Contract

- [x] Task 0.1: Define IR-to-Rust mapping rules and reserved-name policy. Commit as `track 58 task 0.1: define ontology codegen contract`.
- [ ] Task 0.2: Add golden generated output fixtures. Commit as `track 58 task 0.2: add codegen golden fixtures`.

Phase closeout: review, push, and GitHub Actions review.

## Phase 1: Generator

- [ ] Task 1.1: Add failing deterministic regeneration tests. Commit after passing as `track 58 task 1.1: implement deterministic codegen`.
- [ ] Task 1.2: Compile generated components in `kairo-ecs-game-theory`. Commit as `track 58 task 1.2: compile generated game components`.
- [ ] Task 1.3: Add API governance review artifact. Commit as `track 58 task 1.3: record generated api review`.

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
