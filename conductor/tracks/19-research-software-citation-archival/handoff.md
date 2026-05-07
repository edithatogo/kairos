# Handoff: Track 19 Research Software, Citation & Archival

Last updated: 2026-05-07

## Summary

Documented the citation and archival metadata that should accompany a release or published package, with the release-engineering checks tied back to `CITATION.cff`, `codemeta.json`, `.zenodo.json`, `paper/paper.md`, `paper/paper.bib`, the DOI path, release-note links, and the Track 19 validator.

## Files changed by this slice

`.zenodo.json`, `paper/paper.md`, `paper/paper.bib`, `docs/research/citation.md`, `conductor/tracks/19-research-software-citation-archival/test-matrix.md`, `conductor/tracks/19-research-software-citation-archival/handoff.md`, `conductor/tracks/19-research-software-citation-archival/risk-register.md`, `conductor/tracks/19-research-software-citation-archival/validate-citation-archive.ps1`

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
- Evidence: validator reported `version=0.4.0-alpha.1`, `repository=https://github.com/edithatogo/kairos`, and `archive_status=pre-release metadata seed, not yet DOI-minted`.

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
