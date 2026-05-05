# KairoECS Release Checklist

## Preflight

- [ ] Naming/package availability confirmed.
- [ ] Version chosen and release branch created.
- [ ] Changelog updated.
- [ ] Compatibility notes updated.
- [ ] Migration guide written if needed.
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

## Artifacts

- [ ] Native libraries built.
- [ ] C headers included.
- [ ] `dist/release-artifact-manifest.json` generated and reviewed.
- [ ] `dist/SHA256SUMS` generated and reviewed.
- [ ] SBOM generated.
- [ ] Provenance or attestation generated.
- [ ] GitHub Release draft reviewed.
- [ ] Package dry-runs complete for the release set: Rust, Python, R, Julia, TypeScript/Wasm, C#, and Go.
- [ ] Registry dry-runs complete.
- [ ] Release artifacts match the manifest and checksum entries.

## Publish

- [ ] crates.io.
- [ ] PyPI.
- [ ] npm.
- [ ] NuGet.
- [ ] R-universe/CRAN as appropriate.
- [ ] Julia registry/artifacts.
- [ ] Go tag.
- [ ] GitHub Pages.
