# Track 19: Research Software, Citation & Archival

## Purpose

Make KairoECS citable, archivable, and ready for scientific software review and institutional use.

## Why this track exists

KairoECS is not only a Rust kernel. It is a multi-language research and engineering ecosystem. This track protects the project from the most common failure mode for ambitious open-source infrastructure: impressive internals with insufficient trust, examples, packaging, governance, and contributor experience.

## Primary subagent

`research-software-agent`

## Parallelization model

This track is designed to run in parallel with core implementation. The subagent owns docs, policies, examples, checklists, manifests, fixtures, and automation controls. It must not block kernel development unless it identifies a release-blocking risk.

## Inputs

- `CITATION.cff` (current state or gap to fill).
- `Cargo.toml` workspace metadata (authors, version, repository, license).
- `codemeta.json` (current state or gap to fill).
- Handoff notes from Track 16 (release governance, version policy).
- JOSS/software-journal submission guidelines and paper template.

## Outputs

- `CITATION.cff`: validated pre-release metadata with authors, title, version, release date, license, and repository URL. DOI remains absent until a Zenodo draft or minted DOI exists.
- `codemeta.json`: checked-in CodeMeta seed kept synchronized with `CITATION.cff` release version, date, license, and repository URL.
- `paper/`: software-paper seed describing KairoECS architecture, benchmarks, and research contribution without claiming journal submission or acceptance.
- `docs/research/citation.md`: archival plan with the current Zenodo/DOI status, release-archive workflow, and provenance record requirements.
- `conductor/tracks/19-research-software-citation-archival/test-matrix.md`: local gate for `CITATION.cff`, `codemeta.json`, `.zenodo.json`, paper metadata, and release-note citation status.

## Acceptance criteria

- The track has a clear public-facing output, not just internal notes.
- The output is testable, reviewable, or linked to a release gate.
- It includes failure modes and countermeasures.
- It does not duplicate core implementation work owned by Tracks 01-05.
- It supports at least one of: adoption, trust, reproducibility, maintainability, or compatibility.

## Non-goals

- Replacing the core scheduler or ECS design.
- Publishing packages before naming, legal, security, and compatibility gates pass.
- Adding domain-specific complexity to `kairo-ecs-core`.



