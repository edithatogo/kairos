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

- `CITATION.cff`: validated metadata with authors, title, version, doi, repository-url.
- `codemeta.json`: auto-generated from CITATION.cff and Cargo.toml, validated in CI.
- `paper/`: JOSS-formatted software paper describing KairoECS architecture, benchmarks, and research contribution.
- `docs/archival.md`: archival plan with Zenodo DOI reservation, release-archive workflow, and provenance record.
- `conductor/tracks/19-research-software-citation-archival/test-matrix.md`: CI gate (CITATION.cff validation, codemeta.json sync check).

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



