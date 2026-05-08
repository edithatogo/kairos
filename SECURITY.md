# Security Policy

KairoECS includes native code and FFI surfaces. Please report suspected vulnerabilities privately.

## Supported versions

Until 1.0, only the latest release line receives security fixes unless otherwise stated.

## Reporting

Open a private security advisory on GitHub or email the security contact listed by the project maintainers once the repository is public.

Please include:

- affected version/commit
- reproduction steps
- affected language binding if relevant
- expected impact
- whether the issue is public

## Response expectations

Until a public security contact is permanently listed, the private advisory path is the preferred intake route. Maintainers should acknowledge a complete report within 5 business days, assign an owner, and classify severity before any public release note claims the issue is resolved.

For high or critical impact issues, keep discussion private until a fixed release, mitigation, or explicit disclosure decision is available. If the issue affects a published package, the release manager must record the affected package versions, the fixed version or mitigation, and any temporary exception in the release notes.

## Vulnerability policy gates

Release candidates and 1.0 releases must not rely on an informal waiver. Any unresolved vulnerability-policy exception must name the failing control, affected release stage, compensating control, approvers, expiry, and follow-up issue or ADR. Allowed-failure scanner lanes can unblock alpha or beta only when the Track 20 exception process records them as temporary operational exceptions.

## Security-sensitive areas

- FFI handle ownership and lifetime
- callback boundaries
- native library loading
- package publishing workflows
- artifact signing/checksums/provenance
- deserialization of scenario manifests
- visualization/browser sandboxing

## Security controls in the repo

The repository pairs the policy above with concrete workflows and gates:

- `.github/workflows/scorecard.yml` for OpenSSF Scorecard signal
- `.github/workflows/dependency-review.yml` for dependency review on pull requests
- `.github/workflows/sbom-attestations.yml` for SBOM generation and attestation of release artifacts
- `.github/workflows/release-attestations.yml` for release artifact SBOM and attestation
- `.github/workflows/release.yml` for release manifest and checksum generation
- `.github/workflows/package-dry-run.yml` for package dry runs before publication claims
- `.github/workflows/conformance.yml` for validated fixture IDs and benchmark names

Release-facing security claims should name the workflow or artifact they rely on. If a claim depends on SBOM, provenance, or attestation evidence, the release artifact tree must include the generated SBOM and checksum outputs.
