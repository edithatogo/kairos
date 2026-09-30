# 41 Documentation Platform, Quality Gates & Learning Coverage -- plan.md

## Phase 0 -- Track startup

- Read `conductor/workflow.md`.
- Read `conductor/quality-gates.md`, `conductor/track-map.md`,
  `conductor/implementation-readiness.md`, and `docs/community/roadmap.md`.
- Inventory the current CI workflows, docs site implementation, tutorial and
  example surfaces, and notebook assets.
- Confirm owned paths:
  `.github/workflows/`, `docs/`, `examples/`, `notebooks/`, `templates/website/`,
  `website/`, and `scripts/validation/`.
- Create or refresh `agent-contract.md`, `risk-register.md`, `test-matrix.md`,
  and `handoff.md`.

## Phase 1 -- Contract alignment

- Define the strictness contract for existing CI surfaces: warnings-as-errors,
  formatting, linting, typing, validation, and docstring policy where the repo
  already has concrete tooling.
- Define the docs-platform contract for the public site: current coverage,
  route preservation, versioning requirements, and the Astro/Starlight target
  state.
- Define the learning-coverage matrix shape so each supported language and
  example family has a documented tutorial, example, or notebook path.

## Phase 2 -- Scaffold

- Add or tighten the strict CI policy surfaces before changing content
  structure.
- Build the docs migration scaffold or parity plan so the site can move without
  losing current navigation and link validation.
- Create the learning-coverage inventory and identify the first missing
  tutorial/example/notebook items per language.

## Phase 3 -- Implementation

- Harden the repo's existing CI jobs so concrete warnings become failures on
  trusted runs.
- Migrate or align the docs site with Astro/Starlight while preserving current
  docs tree entry points.
- Fill the highest-value tutorial/example/notebook gaps and update the
  coverage matrix as each artifact lands.

## Phase 4 -- Cross-track integration

- Run owned tests and the relevant repo-wide validation commands.
- Update the community and roadmap surfaces so they point at the final docs
  and learning layout.
- Ensure no other subagent-owned paths were modified without handoff.

## Phase 5 -- Closeout

- Complete `handoff.md`.
- Record remaining risks and any deliberate exclusions.
- Confirm CI gates.
- Mark the track ready for integration.

## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next
phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Update the track registry/status surfaces: `conductor/tracks.yaml`
   (authoritative machine-readable registry), `conductor/tracks.md` (human
   index), `conductor/phase-closeout.yaml` (review ledger), and
   `conductor/status.md` (narrative status). Also update
   `conductor/implementation-readiness.md` and `conductor/track-map.md` when
   readiness, ownership, dependency, gate, or wave data changes.
5. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`
   plus the gates listed in `test-matrix.md`.
6. Commit and push the cleaned slice, then record the commit SHA or blocker in
   `handoff.md`.
7. Run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` to verify recorded commits, pushed refs, and cleanup state.
8. Advance the next phase only after there is no in-scope unstaged or
   untracked work except documented draft satellites.
