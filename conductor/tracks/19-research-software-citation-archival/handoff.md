# Handoff: Track 19 Research Software, Citation & Archival

## Summary

Documented the citation and archival metadata that should accompany a release or published package, with the release-engineering checks tied back to `CITATION.cff`, `codemeta.json`, `.zenodo.json`, `paper/paper.md`, `paper/paper.bib`, the DOI path, and release-note links.

## Files changed

`CITATION.cff`, `codemeta.json`, `.zenodo.json`, `paper/paper.md`, `paper/paper.bib`, `docs/research/citation.md`, `conductor/tracks/19-research-software-citation-archival/plan.md`, `conductor/tracks/19-research-software-citation-archival/test-matrix.md`, `conductor/tracks/19-research-software-citation-archival/handoff.md`

## Contracts consumed

`conductor/release-engineering.md`, `conductor/package-catalog.md`, `conductor/delivery-readiness-checklist.md`, `conductor/workflow.md`

## Release gates affected

Citation metadata, archive notes, DOI/Zenodo path, and `just docs-build` now sit on the public-release path before a package is allowed out.

## Concrete citation note

- Citation metadata files: `CITATION.cff`, `codemeta.json`, `.zenodo.json`.
- Paper metadata files: `paper/paper.md`, `paper/paper.bib`.
- DOI/Zenodo path: draft deposition first, then DOI minting for the archived release, with `0.4.0-alpha.1` as the current pre-release citation target.
- Archive notes: release version, archive status, DOI or draft link, source archive location, reproducibility instructions, metadata deltas, repository URL.
- Release notes: version, citation files used, archive status, Zenodo or archive link, reproducibility instructions, and any author/version/repository-code changes.

## Risks and unresolved questions

The concrete risk is stale author, version, or DOI metadata leaking into a release artifact before the archive record is finalized; rerun the citation checks and keep release metadata in sync with the catalog and DOI path.
