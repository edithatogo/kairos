# Track 57 Plan

## Phase 0: Subrepo Skeleton

- [x] Task 0.1: Create `open-game-theory-ontology/` with README, schema directories, provenance policy, and fixture manifest. Commit as `track 57 task 0.1: initialize ontology subrepo`.
- [x] Task 0.2: Add Turtle and JSON-LD minimal schemas for normal-form and extensive-form concepts. Commit as `track 57 task 0.2: add ontology ingestion fixtures`.

Phase closeout: run validators, `$conductor-review 57`, apply fixes, update handoff, push, and review GitHub Actions.

## Phase 1: Parser and Canonical IR

- [x] Task 1.1: Add parser crate and failing tests for Turtle ingestion. Commit only after implementation passes as `track 57 task 1.1: parse turtle ontology fixtures`.
- [x] Task 1.2: Add failing tests and implementation for JSON-LD ingestion. Commit as `track 57 task 1.2: parse jsonld ontology fixtures`.
- [x] Task 1.3: Add deterministic IR normalization and malformed-input tests. Commit as `track 57 task 1.3: normalize ontology ir`.

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
