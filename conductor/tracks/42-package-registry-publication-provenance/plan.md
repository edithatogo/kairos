# 42 Package Registry Publication & Provenance - plan.md

## Phase 0 - Track startup

- [x] Define registry publication ownership.
- [x] Add publication manifest and validation gate.
- [x] Add guarded registry publication workflow.

## Phase 1 - Registry lanes

- [x] Record Rust crates.io lane.
- [x] Record Python TestPyPI/PyPI trusted-publisher lane.
- [x] Record R R-universe/CRAN lane.
- [x] Record Julia dev-registry/General lane.
- [x] Record TypeScript npm provenance lane.
- [x] Record C# NuGet lane.
- [x] Record Go semantic-tag/module proxy lane.

## Phase 2 - SOTA controls

- [x] Require OIDC/trusted publishing where supported.
- [x] Require provenance/attestation, SBOM/checksums, conformance, docs, compatibility notes, and rollback/yank guidance.
- [x] Require Track 44 code health `>= 9.5` before any production registry write.

## Phase 3 - Validation

- [x] Add `scripts/validation/validate-publication-readiness.mjs`.
- [ ] Run publication readiness validation in CI.
- [ ] Complete registry owner/trusted-publisher setup outside the repo.
- [ ] Execute first protected-environment dry run from GitHub Actions.

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
