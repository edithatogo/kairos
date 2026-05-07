# Track 21 Plan: Verification, Validation & Uncertainty

## Phase 0 — Contract alignment

### Task 0.1 — Read existing contracts
- Review the validation, conformance, release, and uncertainty-related notes already present in the repo.
- Identify where this track consumes `conductor/delivery-readiness-checklist.md` and the verification-oriented docs path.
- Open an ADR if this track needs to change a public claim about confidence, uncertainty, or validation scope.

### Task 0.2 — Define owned artifacts
- Keep the work centered on verification checklists, uncertainty notes, and release-facing claims.
- Add owner/subagent to `conductor/subagents.md` if missing.
- Add checks where validation metadata can be verified locally.

## Phase 1 — Minimum viable public artifact

### Task 1.1 — Create the first usable version
- Produce the smallest validation note that explains what has been verified, what remains uncertain, and how to interpret the result.
- Use a concrete KairoECS scenario or fixture reference together with the verified evidence boundary.

### Task 1.2 — Add review criteria
- Add red-team prompts for overclaiming certainty, ignoring edge cases, or mixing validation with proof.
- Add devil's advocate objections about whether the evidence is strong enough for release decisions.
- Add measurable acceptance criteria for confidence levels and documented limitations.

## Phase 2 — Automation and validation

### Task 2.1 — Wire into CI where possible
- Add docs linting, validation-note checks, and fixture or scenario smoke tests where possible.
- Use path guards for any future validation artifacts that are not created yet.

### Task 2.2 — Connect to release gates
- Define what validation evidence is needed before alpha, beta, RC, and 1.0 claims are allowed.
- Add the verification and uncertainty checks to `conductor/delivery-readiness-checklist.md`.

## Phase 3 — Cross-track integration

### Task 3.1 — Handoff to dependent tracks
- Document exactly what other subagents can rely on: verified behaviors, known limits, and the evidence boundary.
- Provide example inputs or fixtures rather than prose-only handoffs.

### Task 3.2 — Add community-facing documentation
- Ensure the docs site has a page explaining how to read uncertainty and validation notes.
- Link from the release page or contributor guide where that guidance will be found first.

## Phase 4 — Closeout

### Task 4.1 — Run quality gates
- Check markdown links.
- Validate the validation-note rendering.
- Run the relevant smoke or dry-run workflow.

### Task 4.2 — Update risk register
- Move resolved risks to mitigated.
- Promote unresolved confidence gaps to release blockers.
## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Update `conductor/phase-closeout.yaml` with review outcome, accepted fixes, validation commands, cleanup state, commit SHA or blocker, pushed ref, and next-phase decision.
5. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` plus the gates listed in `test-matrix.md`.
6. Commit and push the cleaned slice, then record the commit SHA or blocker in `handoff.md`.
7. Run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` to verify recorded commits, pushed refs, and cleanup state.
8. Advance the next phase only after there is no in-scope unstaged or untracked work except documented draft satellites.