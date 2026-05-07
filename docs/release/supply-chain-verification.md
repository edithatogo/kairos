# Verifying Release Artifacts

KairoECS releases should provide a release artifact manifest, checksums, SBOMs,
artifact attestations/provenance, package registry links, and a source archive.
The current dry-run packaging path writes `dist/release-artifact-manifest.json`
and `dist/SHA256SUMS` from `packaging/release-package-manifest.json`, and the
attestation workflows add the SBOM/provenance layer.

Package dry-runs and conformance fixtures are part of the release evidence, not
optional extras. A release should not advance if the package dry-run surface is
failing or if the ready fixture set has drifted. The package dry-run gate should
cover the checked-in package set: Rust, Python, R, Julia, TypeScript/Wasm,
C#, and Go. The current blocker state is dry-run only: registry name
availability is still unverified, and production publish remains disabled until
toolchains and registry targets are confirmed on the release machines.

For local R2 dry-run evidence, run:

```bash
python packaging/scripts/build_release_manifest.py --version 0.0.0-r2-dry-run
```

The generated manifest must keep `production_publish_enabled` set to `false`.
Its output contract is:

- `dist/release-artifact-manifest.json` for the artifact inventory.
- `dist/SHA256SUMS` for the checksum set.

```mermaid
flowchart TD
    Release[GitHub Release]
    Manifest[release-artifact-manifest.json]
    Artifact[Download artifact]
    Checksum[SHA256SUMS]
    Attestation[Release / SBOM attestation]
    SBOM[Review SBOM]
    DryRun[Package dry-run]
    Fixtures[Conformance fixtures]
    Install[Install package]

    Release --> Artifact
    Release --> Manifest
    Artifact --> Checksum
    Artifact --> Attestation
    Artifact --> SBOM
    Manifest --> Checksum
    DryRun --> Release
    Fixtures --> Release
    Checksum --> Install
    Attestation --> Install
```
