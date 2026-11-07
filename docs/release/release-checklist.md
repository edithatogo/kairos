# KairoECS Release Checklist

This checklist is used with the [beta and stable gates](../../conductor/release-gates.md)
and [capability register](../../conductor/capabilities.json). A completed track or
an old green workflow is not acceptance of the current release candidate.

## Preflight

- [ ] Naming/package availability confirmed.
- [ ] Version chosen and release branch created.
- [ ] Changelog updated.
- [ ] Compatibility notes updated.
- [ ] Changelog policy check passed for public release-surface changes.
- [ ] Deprecation register reviewed; removals have prior notice or an explicit approved breaking-change ADR, including immediate Rust-only execution in ADR 0010.
- [ ] Migration guide written if needed.
- [ ] Maintainer rotation assigned in `docs/release/maintainer-rotation.md`.
- [ ] R2 package inventory reviewed: `packaging/release-package-manifest.json`.
- [ ] Dry-run path confirmed: `python packaging/scripts/build_release_manifest.py --check` then `python packaging/scripts/build_release_manifest.py --version 0.0.0-r2-dry-run`.
- [ ] Release artifact manifest path agreed: `dist/release-artifact-manifest.json`.
- [ ] Checksum manifest path agreed: `dist/SHA256SUMS`.

## Quality

- [ ] Rust core CI green at the exact candidate commit, lockfiles and feature matrix.
- [ ] Each advertised adapter loads the shipped Rust library or Rust-generated Wasm module and executes its promised operations in an installed consumer.
- [ ] Missing/invalid native libraries, incompatible ABI versions and unsupported operations fail explicitly without host-language fallback.
- [ ] Python 3.15.2 or a specifically qualified compatible profile passes native acceptance; metadata/import-only checks are labelled separately.
- [ ] R, Julia, TypeScript/Wasm, C# and Go are individually qualified or explicitly excluded from the release support matrix. An excluded adapter is not advertised as supported.
- [ ] Native handle lifetime, cancellation, buffer ownership, repeatability and error paths pass for each included adapter.
- [ ] Scientific methods satisfy their independent analytical/differential oracle gates; known mismatches are removed from supported claims or fixed before promotion.
- [ ] Conformance suite green.
- [ ] Conformance fixture manifest is current and the ready fixtures are still valid.
- [ ] Benchmarks reviewed.
- [ ] Performance claims link raw measurements, exact workload/backend/hardware metadata and a suitable baseline; generated fixtures are never counted as measurements.
- [ ] Correctness and model validation are recorded separately from throughput and coverage.
- [ ] Security scans reviewed.
- [ ] `release.yml`, `release-attestations.yml`, and `sbom-attestations.yml` are all passing on the release branch.

## Docs

- [ ] GitHub Pages build green.
- [ ] API docs generated.
- [ ] Quickstarts match the supported native operations; unavailable language operations have explicit support boundaries.
- [ ] Release notes published.
- [ ] Maintenance handoff completed.
- [ ] Maintainer rotation and escalation path reviewed.

## Artifacts

- [ ] Native libraries built, checksummed and exercised from clean installed consumers on every advertised platform.
- [ ] C headers included.
- [ ] Local manifest/checksum builder passed: `python packaging/scripts/build_release_manifest.py --check`.
- [ ] `dist/release-artifact-manifest.json` generated and reviewed.
- [ ] `dist/SHA256SUMS` generated and reviewed.
- [ ] Generated release evidence verified: `python packaging/scripts/build_release_manifest.py --verify-existing`.
- [ ] Dry-run output reviewed for package inventory, registry modes, and checksum coverage.
- [ ] Valid SPDX 2.3 SBOM generated at `dist/sbom.spdx.json`.
- [ ] Provenance generated at `dist/provenance.json` or `dist/provenance.intoto.jsonl` and covers every SHA-256 subject in the generated release manifest.
- [ ] `dist/SUPPLY-CHAIN-SHA256SUMS` verifies the SBOM and provenance files.
- [ ] GitHub Release draft reviewed.
- [ ] Package dry-runs complete for the explicitly selected release set; unsupported adapters are excluded rather than represented by metadata-only success.
- [ ] Registry dry-run matrix reviewed against `packaging/release-package-manifest.json`.
- [ ] Registry dry-runs complete.
- [ ] Any registry that remains dry-run-only has an explicit blocker note.
- [ ] Release artifacts match the manifest and checksum entries.
- [ ] Release evidence copied into `docs/release/maintenance-handoff.md` or linked from it.
- [ ] Any remaining publish blockers are recorded in the maintenance handoff before leaving dry-run mode.

Historical R2 local dry-run evidence, 2026-05-08 (not candidate acceptance):

- `python packaging/scripts/build_release_manifest.py --version 0.0.0-r2-dry-run` generated `dist/release-artifact-manifest.json` and `dist/SHA256SUMS`.
- Track 15 dry-run validation reported `track15_status=ok`.
- This is local reproducible dry-run evidence only; public publishing remains blocked until registry/name/toolchain checks are complete.

## Publish

Record an explicit supported/not-in-release decision for each surface. Publish
only the approved release set after its native and supply-chain gates pass.


- [ ] crates.io.
- [ ] PyPI.
- [ ] npm.
- [ ] NuGet.
- [ ] R-universe/CRAN as appropriate.
- [ ] Julia registry/artifacts.
- [ ] Go tag.
- [ ] GitHub Pages.
