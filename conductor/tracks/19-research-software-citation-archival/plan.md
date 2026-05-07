# Track 19 Plan: Research Software, Citation & Archival

## Phase 0 — Contract alignment

### Task 0.1 — Read existing contracts
- Review the release, metadata, and archival surfaces that support citation and long-term preservation.
- Identify where this track consumes `conductor/release-engineering.md`, the package catalog, and the concrete citation files: `CITATION.cff`, `codemeta.json`, and `.zenodo.json`.
- Open an ADR if a citation or archival claim would alter a published release promise.

### Task 0.2 — Define owned artifacts
- Keep the work centered on citation metadata, archival notes, and release packaging references.
- Add owner/subagent to `conductor/subagents.md` if missing.
- Add checks where the citation or archive metadata can be validated locally.

## Phase 1 — Minimum viable public artifact

### Task 1.1 — Create the first usable version
- Produce the smallest citation-and-archive guide that explains how KairoECS should be referenced and preserved.
- Use the checked-in metadata fields as the real citation target:
  - `CITATION.cff`: `cff-version`, `message`, `title`, `type`, `authors`, `abstract`, `keywords`, `license`, `repository-code`
  - `codemeta.json`: `@context`, `@type`, `name`, `description`, `programmingLanguage`, `license`, `codeRepository`, `developmentStatus`
  - `.zenodo.json`: `title`, `upload_type`, `description`, `creators`, `license`, `keywords`
- Use one concrete release or package example with a real DOI path instead of placeholder metadata.
- Anchor the first archived release to the pre-release plan already named in release engineering: `0.4.0-alpha.1`.
- Keep the repo code URL consistent across the citation files, paper metadata, and the release note.

### Task 1.2 — Add review criteria
- Add red-team prompts for incomplete author metadata, missing version references, and ambiguous archival claims.
- Add devil's advocate objections about whether the citation record is durable enough for reuse.
- Add measurable acceptance criteria for fields that must be present before release.

## Phase 2 — Automation and validation

### Task 2.1 — Wire into CI where possible
- Add citation metadata validation, archive manifest checks, and docs linting where possible.
- Use path guards for archive or release outputs that are not built yet.

### Task 2.2 — Connect to release gates
- Define what citation and archival evidence is required before alpha, beta, RC, and 1.0.
- Add the archival checks to the release note and archive record path documented in `conductor/release-engineering.md`.

## Phase 3 — Cross-track integration

### Task 3.1 — Handoff to dependent tracks
- Document exactly what other subagents can rely on: citation fields, archive metadata, DOI/Zenodo path, and release references.
- Provide concrete metadata examples rather than prose-only handoffs.

### Task 3.2 — Add community-facing documentation
- Ensure the docs site has a page explaining how to cite and archive the project.
- Link from the release page, contributor guide, or package catalog as appropriate.
- Keep the page aligned with `paper/paper.md` and `paper/paper.bib`.

## Phase 4 — Closeout

### Task 4.1 — Run quality gates
- Check markdown links.
- Validate metadata linting or archive preview output.
- Run the release metadata smoke check where available.

### Task 4.2 — Update risk register
- Move resolved risks to mitigated.
- Keep unresolved citation ambiguity or archival gaps as blockers for any public release claim.
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