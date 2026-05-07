# Track 23 Plan: Domain Starter Kits & Model Zoo

## Phase 0 — Contract alignment

### Task 0.1 — Read existing contracts
- Review the model-zoo and starter-kit surfaces that already exist in the repo layout.
- Identify where this track consumes `website/`, `conductor/package-catalog.md`, and release-facing docs.
- Open an ADR if a starter kit would imply a new public compatibility promise.

### Task 0.2 — Define owned artifacts
- Keep the work centered on starter-kit listings, model-zoo entries, and example packaging notes.
- Add owner/subagent to `conductor/subagents.md` if missing.
- Add checks where the starter-kit inventory can be verified locally.

## Phase 1 — Minimum viable public artifact

### Task 1.1 — Create the first usable version
- Produce the smallest starter-kit or model-zoo index that shows how a new user gets from docs to a runnable example.
- Use one concrete KairoECS example path that can be linked from the model-zoo index.

### Task 1.2 — Add review criteria
- Add red-team prompts for stale kit listings, mismatched dependencies, and unsupported examples.
- Add devil's advocate objections about whether the model zoo is actually useful as a discovery layer.
- Add measurable acceptance criteria for inventory completeness and linkability.

## Phase 2 — Automation and validation

### Task 2.1 — Wire into CI where possible
- Add docs linting, inventory checks, and smoke validation for any published example links.
- Use path guards for future starter kits that are not created yet.

### Task 2.2 — Connect to release gates
- Define what starter-kit or model-zoo evidence is required before alpha, beta, RC, and 1.0 claims.
- Add the model-zoo checks to `conductor/delivery-readiness-checklist.md`.

## Phase 3 — Cross-track integration

### Task 3.1 — Handoff to dependent tracks
- Document exactly what other subagents can rely on: kit names, example paths, and model-zoo entry points.
- Provide example paths and inventory records rather than prose-only handoffs.

### Task 3.2 — Add community-facing documentation
- Ensure the docs site has a page explaining how to discover starter kits and reference examples.
- Link from the docs index, package catalog, or contributor guide as appropriate.

## Phase 4 — Closeout

### Task 4.1 — Run quality gates
- Check markdown links.
- Validate the starter-kit index renders cleanly.
- Run the docs build or inventory smoke test.

### Task 4.2 — Update risk register
- Move resolved risks to mitigated.
- Keep unresolved example drift or broken starter-kit links as blockers for the public catalog.
## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` plus the gates listed in `test-matrix.md`.
5. Commit and push the cleaned slice, then record the commit SHA or blocker in `handoff.md`.
6. Advance the next phase only after there is no in-scope unstaged or untracked work except documented draft satellites.