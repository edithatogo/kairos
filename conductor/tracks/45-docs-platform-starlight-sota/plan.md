# 45 Astro/Starlight Docs Platform and Polyglot Experience - plan.md

## Phase 0 - Track startup

- [x] Add Track 45 to the Conductor registry.
- [x] Record docs-platform ownership and dependencies.
- [x] Fix Track 44 `>= 9.5` metadata drift while touching the registry.

## Phase 1 - Active platform contract

- [x] Confirm Astro/Starlight is the active docs shell.
- [x] Confirm `starlight-versions` is wired with the `R2 Preview` and `R1 Archive` route.
- [x] Confirm the local `kairoecs-starlight-polyglot` plugin is wired.
- [x] Confirm SOTA helper plugins: link validation, llms.txt, icons, and generated Pagefind search output.

## Phase 2 - Validation and CI

- [x] Add `scripts/validation/validate-docs-platform-sota.mjs`.
- [x] Add `npm --prefix website run check:sota`.
- [x] Wire the SOTA validator into `.github/workflows/docs-quality.yml`.
- [x] Update docs-platform status and deferred-plugin guidance.

## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Update the track registry/status surfaces: `conductor/tracks.yaml` (authoritative machine-readable registry), `conductor/tracks.md` (human index), `conductor/phase-closeout.yaml` (review ledger), `conductor/status.md` (narrative status), and `conductor/implementation-readiness.md` or `conductor/track-map.md` when readiness, ownership, dependency, gate, or wave data changes.
5. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` plus the gates listed in `test-matrix.md`.
6. Commit and push the cleaned slice, then record the commit SHA or blocker in `handoff.md`.
7. Run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` to verify recorded commits, pushed refs, and cleanup state.
8. Advance the next phase only after there is no in-scope unstaged or untracked work except documented draft satellites.
