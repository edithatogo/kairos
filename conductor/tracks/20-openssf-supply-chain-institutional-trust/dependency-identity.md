# Dependency graph alias identity correction

GitHub reports the npm install key http-cache-semantics@0.1.0 for the private scoped package, and therefore assigns upstream advisories. This correction accepts only the exact bootstrap manifest, install key, version, purl, BSD license and null upstream repository, the two observed advisories, and pinned full lockfile/archive/source bytes. Every other high/critical finding fails. All pages are inspected; unknown schema or missing API proof fails. Raw graph and classification are retained.

The action retains license checking; its vulnerability stage is replaced by the explicit equivalent high/critical control above it. There is no blanket advisory allowance, severity reduction, audit waiver or upstream version claim. Package CI independently requires zero npm findings, strict registry signatures, consumer source checks and both60/248 suites. This control records an identity mismatch, not a clean upstream scan or waiver approval.
