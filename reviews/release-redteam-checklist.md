# Release Red-Team Checklist

Before any public release, answer:

- Are claims limited to implemented and tested behavior?
- Does every published package pass conformance fixtures?
- Are package names verified across registries?
- Is there a rollback/unpublish policy for each registry?
- Are native artifacts signed or at least checksummed?
- Is the SBOM attached?
- Is provenance attached where feasible?
- Are generated bindings reproducible?
- Are docs versioned with the release?
- Are known limitations explicit?
- Are benchmark claims linked to reproducible scripts?
- Is .NET 11 marked preview if still preview?
- Are Python 3.10-3.14 wheels smoke-tested or clearly source-only for unsupported platforms?
- Is the red-team freshness date within 14 days or re-run for the target release stage?
- Does every blocker or warning have a named owner, evidence path, and stage impact?
- Are release artifact claims withheld until `dist/release-artifact-manifest.json`, `dist/SHA256SUMS`, SBOM, and provenance evidence exist?
- Are planned conformance fixture IDs kept out of beta/RC/1.0 capability claims?
