# KairoECS Release Checklist

## Preflight

- [ ] Naming/package availability confirmed.
- [ ] Version chosen and release branch created.
- [ ] Changelog updated.
- [ ] Compatibility notes updated.
- [ ] Changelog policy check passed for public release-surface changes.
- [ ] Deprecation register reviewed; removals have prior notice or ADR.
- [ ] Migration guide written if needed.
- [ ] R2 package inventory reviewed: `packaging/release-package-manifest.json`.
- [ ] Release artifact manifest path agreed: `dist/release-artifact-manifest.json`.
- [ ] Checksum manifest path agreed: `dist/SHA256SUMS`.

## Quality

- [ ] Rust core CI green.
- [ ] Python 3.10-3.14 green.
- [ ] R green.
- [ ] Julia green.
- [ ] TypeScript/Wasm green.
- [ ] C# .NET 10-11 green or .NET 11 marked experimental with rationale.
- [ ] Go green.
- [ ] Conformance suite green.
- [ ] Conformance fixture manifest is current and the ready fixtures are still valid.
- [ ] Benchmarks reviewed.
- [ ] Security scans reviewed.
- [ ] `release.yml`, `release-attestations.yml`, and `sbom-attestations.yml` are all passing on the release branch.

## Docs

- [ ] GitHub Pages build green.
- [ ] API docs generated.
- [ ] Quickstarts for all languages updated.
- [ ] Release notes published.
- [ ] Maintenance handoff completed.

## Artifacts

- [ ] Native libraries built.
- [ ] C headers included.
- [ ] Local manifest/checksum builder passed: `python packaging/scripts/build_release_manifest.py --check`.
- [ ] `dist/release-artifact-manifest.json` generated and reviewed.
- [ ] `dist/SHA256SUMS` generated and reviewed.
- [ ] SBOM generated.
- [ ] Provenance or attestation generated.
- [ ] GitHub Release draft reviewed.
- [ ] Package dry-runs complete for the release set: Rust, Python, R, Julia, TypeScript/Wasm, C#, and Go.
- [ ] Registry dry-run matrix reviewed against `packaging/release-package-manifest.json`.
- [ ] Registry dry-runs complete.
- [ ] Any registry that remains dry-run-only has an explicit blocker note.
- [ ] Release artifacts match the manifest and checksum entries.
- [ ] Release evidence copied into `docs/release/maintenance-handoff.md` or linked from it.

## Publish

- [ ] crates.io.
- [ ] PyPI.
- [ ] npm.
- [ ] NuGet.
- [ ] R-universe/CRAN as appropriate.
- [ ] Julia registry/artifacts.
- [ ] Go tag.
- [ ] GitHub Pages.
