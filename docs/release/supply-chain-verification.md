# Verifying Release Artifacts

KairoECS releases should provide a release artifact manifest, checksums, SBOMs,
artifact attestations/provenance, package registry links, and a source archive.
The release workflow writes `dist/release-artifact-manifest.json` and
`dist/SHA256SUMS` for the checked-in package surfaces it knows about, and the
attestation workflows add the SBOM/provenance layer.

Package dry-runs and conformance fixtures are part of the release evidence, not
optional extras. A release should not advance if the package dry-run surface is
failing or if the ready fixture set has drifted. The package dry-run gate should
cover the checked-in package set: Rust, Python, R, Julia, TypeScript/Wasm,
C#, and Go.

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
