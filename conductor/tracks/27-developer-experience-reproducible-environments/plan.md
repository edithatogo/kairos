# Track 27 Plan: Developer Experience & Reproducible Environments

## Phase 0 — Contract alignment

### Task 0.1 — Read existing contracts
- Review the docs workflow, local preview commands, and reproducibility surfaces already present in the repo.
- Identify where this track consumes `justfile`, `website/`, and `conductor/workflow.md`.
- Open an ADR if the bootstrap or preview commands would change a published contributor promise.

### Task 0.2 — Define owned artifacts
- List files owned by this track.
- Add owner/subagent to `conductor/subagents.md` if missing.
- Add CI/release gates if the artifact can be machine-checked.
- Include the contributor commands needed to bootstrap and preview `website/`.

## Phase 1 — Minimum viable public artifact

### Task 1.1 — Create the first usable version
- Produce the smallest contributor-facing note that makes `just docs-bootstrap`, `just docs-build`, and `just docs-dev` discoverable.
- Use the existing `website/` and `justfile` commands rather than a generic example.
- Make `just dev-setup`, `just docs-bootstrap`, `just docs-build`, and `just docs-dev` runnable against the current repository layout.

### Task 1.2 — Add review criteria
- Add red-team prompts for broken local setup, hidden prerequisites, and docs-preview drift.
- Add devil's advocate objections about whether the workflow is actually reproducible on a clean machine.
- Add measurable acceptance criteria for command output and preview availability.

## Phase 2 — Automation and validation

### Task 2.1 — Wire into CI where possible
- Add docs linting, local preview smoke tests, and path-guarded checks for the current site layout.
- Use path guards for not-yet-created packages.
- Keep docs build checks local to `website/` so they do not pretend to validate unimplemented packages.

### Task 2.2 — Connect to release gates
- Define what contributor-experience evidence is required before alpha, beta, RC, and 1.0 claims.
- Add the bootstrap and preview commands to `conductor/delivery-readiness-checklist.md`.

## Phase 3 — Cross-track integration

### Task 3.1 — Handoff to dependent tracks
- Document exactly what other subagents can rely on: bootstrap commands, preview ports, and site paths.
- Provide command examples and site paths rather than prose-only handoffs.

### Task 3.2 — Add community-facing documentation
- Ensure the docs site has a page explaining the local workflow and preview path.
- Link from the contributor guide and docs index where the commands are discoverable.

## Phase 4 — Closeout

### Task 4.1 — Run quality gates
- Check markdown links.
- Validate the docs bootstrap and preview commands.
- Run the local docs build or preview smoke workflow.

### Task 4.2 — Update risk register
- Move resolved risks to mitigated.
- Promote unresolved setup or preview drift to release blockers.
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