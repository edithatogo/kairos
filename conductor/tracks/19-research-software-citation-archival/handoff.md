# Handoff: Track 19 Research Software, Citation & Archival

Last updated: 2026-05-11

## Summary

Documented the citation and archival metadata that should accompany a release or published package, with the release-engineering checks tied back to `CITATION.cff`, `codemeta.json`, `.zenodo.json`, `paper/paper.md`, `paper/paper.bib`, the DOI path, release-note links, and the Track 19 validator.

## Files changed by this slice

`codemeta.json`, `.zenodo.json`, `paper/paper.md`, `paper/paper.bib`, `docs/research/citation.md`, `conductor/tracks/19-research-software-citation-archival/test-matrix.md`, `conductor/tracks/19-research-software-citation-archival/handoff.md`, `conductor/tracks/19-research-software-citation-archival/risk-register.md`, `conductor/tracks/19-research-software-citation-archival/validate-citation-archive.ps1`

## Contracts consumed

`conductor/release-engineering.md`, `conductor/package-catalog.md`, `conductor/delivery-readiness-checklist.md`, `conductor/workflow.md`

## Release gates affected

Citation metadata, archive notes, DOI/Zenodo path, and `just docs-build` now sit on the public-release path before a package is allowed out.

## Concrete citation note

- Citation metadata files: `CITATION.cff`, `codemeta.json`, `.zenodo.json`.
- Paper metadata files: `paper/paper.md`, `paper/paper.bib`.
- DOI/Zenodo path: draft deposition first, then DOI minting for the archived release, with `0.4.0-alpha.1` as the current pre-release citation target. Current status is `pre-release metadata seed, not yet DOI-minted`.
- Archive notes: release version, archive status, DOI or draft link, source archive location, reproducibility instructions, metadata deltas, repository URL.
- Release notes: version, citation files used, archive status, Zenodo or archive link, reproducibility instructions, and any author/version/repository-code changes.

## Local validation evidence

- Command: `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/19-research-software-citation-archival/validate-citation-archive.ps1`
- Result: pass.
- Evidence: validator reported `version=0.4.0-alpha.1`, `repository=https://github.com/edithatogo/kairos`, and `archive_status=pre-release metadata seed, not yet DOI-minted`. The validator now normalizes SPDX license URLs and checks the CodeMeta license against `CITATION.cff`.
- Command: `just check-docs`
- Result: pass.
- Command: `just docs-build`
- Result: pass.
- Command: `node tests/conformance/track12_20_evidence_check.mjs`
- Result: pass for Track 19 inside the aggregate Track 12-20 evidence gate.
- Command: `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`
- Result: pass.
- Command: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1`
- Result: pass.
- Field checks for `CITATION.cff`, `.zenodo.json`, `codemeta.json`, and `paper/` metadata passed.

## Risks and unresolved questions

The concrete risk is stale author, version, or DOI metadata leaking into a release artifact before the archive record is finalized; rerun `validate-citation-archive.ps1` and keep release metadata in sync with the catalog and DOI path.

## Contracts changed

No citation or archive contracts changed in this scoped cleanup; the authoritative metadata remains `CITATION.cff`, `codemeta.json`, `.zenodo.json`, and paper metadata.

## Tests added

No executable tests were added in this scoped cleanup. Existing evidence remains `validate-citation-archive.ps1`.

## Known risks

The release is still pre-DOI; metadata must remain marked as not DOI-minted until a Zenodo draft or DOI exists.

## Follow-up issues

Reserve or record the DOI before public release and rerun the citation/archive validator after any version, author, repository, title, license, or archive metadata change.

## Integration notes

Do not use Track 19 metadata as release authorization by itself; it supports release evidence once packaging, governance, and trust gates also pass.
## Phase closeout evidence

2026-05-11 implementation/review pass:

- `$conductor-review` finding fixed: `codemeta.json` used the CodeMeta 2.0 context while `conductor/metadata-check.md` requires the CodeMeta 3.0 crosswalk; `validate-citation-archive.ps1` now enforces the 3.0 context.
- Accepted fixes applied inside Track 19 ownership: `codemeta.json`, Track 19 validator, test matrix, handoff, registry/status entries.
- Validation commands passed: Track 19 citation/archive validator, `just check-docs`, `just docs-build`, aggregate Track 12-20 evidence gate, Conductor phase-gate validator, Conductor DAG validator, and focused metadata field checks.
- Commit/push state: blocked; no Track 19 commit was created because the shared worktree still has uncommitted tracked or untracked changes outside this track.
- Pushed ref: blocked; no push was performed because `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` failed on the dirty tree.
- Strict git closeout: `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` fails until the worktree is clean again and the closeout evidence can be recorded.
- Next-phase decision: Track 19 remains `In Review`; do not move it to `Done` until the clean-tree closeout gate passes.

- commit SHA: `19001a631f4cc7099236670daafd0123d1575a31`
- pushed ref: `origin/conductor-close-reviewed-tracks-20260510`
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: passed for the repository clean tree after the closeout commit was recorded and pushed.
- next-phase decision: Track 19 is `Done`; keep citation/archive metadata bounded to the existing pre-release seed until a DOI-minted archive record exists.


## PR #95 registry roadmap reconstruction — 2026-10-01

The registry planning scope is integrated into the existing numbered Track 19. `docs/research/registry-readiness.md` maps #90 and #91–#94 to licensing/release metadata, Software Heritage, RRID, and JOSS prerequisites and authoritative completion evidence. Track 19 spec, plan and test matrix link the roadmap; CHANGELOG records the documentation addition. Original PR #95's replacement of global product, technology, workflow and track registries and its duplicate unnumbered track are excluded. No track status, dependency, ownership or phase advances.

Review: every destination has a prerequisite and an external completion gate. Readiness, project membership, archive IDs, journal acceptance and submission are not inferred from preparation; #90–#94 remain open follow-ups. Existing citation metadata and pre-DOI status are preserved.

Validation from `/private/tmp/kairos-pr95-integration` on base `b194277e675c7d1332f8b7393bd4b21af661d066`, with Node 26.10.0, npm 11.19.1, PowerShell 7.6.6 and Rust 1.98.1:

- `pwsh -NoProfile -File conductor/tracks/19-research-software-citation-archival/validate-citation-archive.ps1`: exit 0, existing `0.4.0-alpha.1` metadata seed remains not DOI-minted.
- `pwsh -NoProfile -File scripts/validate_conductor_setup.ps1`: exit 0, numbered track structure, no-skip claims, phase/git ledger checks and its workspace tests passed.
- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`: exit 0, zero errors/warnings.
- `just check-docs`: first attempt failed because Astro was absent in the fresh worktree; after `npm ci --prefix website --ignore-scripts --no-audit --no-fund` installed the checked-in lock, rerun exited 0. Build, internal links and HTTP smoke passed; generated artifact: `website/build/`. Existing empty i18n/404 content warnings remained.
- Focused relative-link readback for the new roadmap and changed Track 19 spec/plan: all referenced local targets resolve.
- `git diff --check`: exit 0.

Hosted Actions, provider eligibility/readbacks and native issue/project reconciliation remain pending. This local planning slice authorizes no external registry submission or publication.
