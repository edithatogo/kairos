# Track 56 Plan

## Phase 0: Charter and Evidence Contract

- [x] Task 0.1: Add `conductor/game-theory-ontology-wave.md` with parity targets, evidence policy, release wording rules, and dependency order. Commit as `track 56 task 0.1: add game theory wave charter`.
- [ ] Task 0.2: Add `conductor/game-theory-evidence/schema.json` and template manifests for Tracks 57-61. Commit as `track 56 task 0.2: add ontology evidence schema`.
- [ ] Task 0.3: Add a validator that rejects missing task commits, missing phase review/push records, and missing GitHub Actions review evidence. Commit as `track 56 task 0.3: validate game theory evidence gates`.

Phase closeout: run Conductor validators, `$conductor-review 56`, apply accepted fixes, update this plan and `handoff.md`, push, then run `gh pr checks --watch` or record why no PR/check surface exists.

## Phase 1: Claim Boundary Enforcement

- [ ] Task 1.1: Add claim-boundary scan rules for ontology, graph relations, normal-form games, and extensive-form games. Commit as `track 56 task 1.1: add game theory claim boundary scan`.
- [ ] Task 1.2: Add negative fixtures proving scaffold-only evidence cannot satisfy production claims. Commit as `track 56 task 1.2: add negative evidence fixtures`.

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
