# Track 17 Plan: Community Adoption, Education & Ecosystem

## Phase 0 — Contract alignment

### Task 0.1 — Read existing contracts
- Review `website/src/index.md`, `conductor/package-catalog.md`, `conductor/delivery-readiness-checklist.md`, and the community-facing track docs.
- Identify where this track consumes or references the checked-in docs site commands: `just docs-build`, `just check-docs`, and `cd website && npm ci && npm run build`.
- Open an ADR if any community claim would depend on unpublished APIs, registries, or release timing.

### Task 0.2 — Define owned artifacts
- Keep the work centered on `website/src/index.md`, `conductor/package-catalog.md`, `conductor/delivery-readiness-checklist.md`, onboarding copy, and ecosystem index links.
- Do not introduce a new docs root or package surface; use the checked-in `website/` tree and the current binding/package entries already listed in `conductor/package-catalog.md`.
- Add CI or docs gates only for artifacts that can be checked locally with `just docs-build` and `just check-docs`.

## Phase 1 — Minimum viable public artifact

### Task 1.1 — Create the first usable version
- Produce the smallest community-facing guide that helps a newcomer understand where KairoECS documentation, examples, package surfaces, and release notes live.
- Use the existing `website/src/index.md` docs surface and `conductor/package-catalog.md` as the first publishing surface rather than inventing a new site path.

### Task 1.2 — Add review criteria
- Add red-team prompts for misleading onboarding, stale links, and unsupported claims about ecosystem maturity.
- Add devil's advocate objections about discoverability, maintenance burden, and whether the community page actually helps a new contributor.
- Add measurable acceptance criteria for link integrity and content freshness.

## Phase 2 — Automation and validation

### Task 2.1 — Wire into CI where possible
- Add markdown linting, broken-link checks, and docs-build smoke tests for the community pages using the same local gates surfaced in `justfile`.
- Use path guards for future examples or starter repos that are not created yet.

### Task 2.2 — Connect to release gates
- Define what community materials must be present before alpha, beta, and 1.0 can be announced publicly.
- Add the relevant docs checks to `conductor/delivery-readiness-checklist.md`.

## Phase 3 — Cross-track integration

### Task 3.1 — Handoff to dependent tracks
- Document which paths, labels, and link targets other subagents can rely on.
- Provide actual doc targets and catalog entries rather than prose-only handoffs.

### Task 3.2 — Add community-facing documentation
- Ensure `website/src/index.md` includes a visible entry point for adoption, education, and ecosystem navigation.
- Link to the model zoo, API review, release notes, contributor guide, and the package catalog sections where those surfaces already exist.

## Phase 4 — Closeout

### Task 4.1 — Run quality gates
- Check markdown links.
- Validate the docs site build with `just docs-build`.
- Run `just check-docs` to confirm the rendered site and index links stay in sync.

### Task 4.2 — Update risk register
- Move resolved risks to mitigated.
- Keep unresolved discoverability or maintenance risks as release blockers until the docs surface proves stable.
## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` plus the gates listed in `test-matrix.md`.
5. Commit and push the cleaned slice, then record the commit SHA or blocker in `handoff.md`.
6. Advance the next phase only after there is no in-scope unstaged or untracked work except documented draft satellites.