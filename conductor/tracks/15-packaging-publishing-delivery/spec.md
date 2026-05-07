# 15 Packaging, Publishing & Delivery — spec.md

## Mission

Define the non-destructive packaging and registry plan for Rust, Python, R, Julia, TypeScript, C#, and Go.
The track is planning-first: it records package names, required artifacts, registry order, and dry-run gates before any production publish is attempted.

## Primary subagent

```text
release-agent + binding agents
```

## Dependencies

```text
Binding package surfaces; registry availability checks; naming approval; release policy approval.
```

## Owned paths

```text
conductor/package-matrix.md
conductor/package-catalog.md
conductor/release-engineering.md
conductor/tracks/15-packaging-publishing-delivery/*
```

## Parallel-safe with

Most tracks are parallel-safe after their contract inputs are accepted. See `conductor/parallel-execution.md` for the wave model.

## Inputs

- `conductor/package-matrix.md` and `conductor/package-catalog.md`.
- `conductor/contracts/ffi-contract.md` (FFI binding surfaces that become packages).
- `conductor/contracts/naming-contract.md` and naming approval record.
- Handoff notes from Tracks 02-11 (binding agent outputs per language).
- Registry availability checks for crates.io, PyPI, npm, NuGet, Go proxy.

## Outputs

- `conductor/package-matrix.md`: updated with registry reservations and fallback names per ecosystem.
- `conductor/package-catalog.md`: per-crate/per-package metadata (name, version, registry target, artifact type).
- `conductor/release-engineering.md`: dry-run sequence, signing requirements, and publish order.
- `conductor/tracks/15-packaging-publishing-delivery/test-matrix.md`: CI gate definitions (pack, check, dry-run per ecosystem).

## Publishing scope

Publishing must remain staged and dry-run only until the release gates are satisfied:

```text
Rust: cargo package + cargo publish --dry-run
Python: build + twine check + TestPyPI dry-run
R: R CMD build/check + R-universe/GitHub artifact build
Julia: package metadata + artifact validation + dev registry prep
TypeScript: npm pack + npm publish --dry-run
C#: dotnet pack + local NuGet validation
Go: module metadata + semantic tag plan, no pushed tag yet
GitHub Releases: draft only until gates pass
GitHub Pages: build-only until docs gate passes
```

## Acceptance criteria

- Package names are listed with fallback notes where applicable.
- Each ecosystem has a first artifact, registry target, and dry-run action.
- Track tests or validation checks exist.
- CI gate is defined at a documentation level.
- Documentation impact is recorded.
- Release implications are recorded.
- `handoff.md` is completed before merge.

## Quality gates

Use the gates in `conductor/quality-gates.md`. Track-specific gates must be listed in `test-matrix.md`.

## Blocked paths

No additional blocked paths are declared for this track beyond the ownership and dependency boundaries in conductor/tracks.yaml. Public release, packaging, or production-readiness claims remain blocked until the relevant downstream release gates pass or are explicitly waived.


## Release implications

This track contributes to release readiness only through the acceptance criteria and quality gates listed here and in conductor/quality-gates.md. It does not independently authorize public release, registry publication, or production-readiness claims without the dependent packaging, supply-chain, compatibility, red-team, and wave-management gates.
