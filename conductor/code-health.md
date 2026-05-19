# KairoECS Code and Repository Health Score

Target: `>= 9.5/10` before any production registry write, beta, RC, 1.0, or production-ready cloud/HPC claim.

This score is deliberately stricter than green CI. It combines code quality, repository hygiene, documentation quality, supply-chain health, release maturity, and ecosystem/package readiness. A category can be green locally and still fail the score if the evidence is not release-grade.

## Current score target

| Category | Weight | Current score | Minimum required |
|---|---:|---:|---:|
| CI and tests | 2.0 | 2.0 | 1.9 |
| Security and supply chain | 2.0 | 2.0 | 1.9 |
| Docs and learning coverage | 1.5 | 1.5 | 1.425 |
| Release and registry readiness | 1.5 | 1.5 | 1.425 |
| API compatibility and conformance | 1.5 | 1.5 | 1.425 |
| Repo hygiene and maintainability | 1.5 | 1.5 | 1.425 |

Overall release threshold: `9.5`.

## Required evidence

- All required GitHub Actions pass on `main`.
- Open PR and issue queues are empty or explicitly triaged for the target release.
- No untracked generated artifacts are required for the release.
- Starlight/Astro docs build, link validation, versioning, llms.txt, and desktop/mobile smoke evidence are current.
- Every package lane has SOTA publication controls: OIDC or narrow scoped credentials, provenance or attestation where supported, SBOM/checksum references, conformance evidence, compatibility notes, and rollback/yank guidance.
- Every cloud/HPC lane records whether evidence is offline-only or live runtime-backed.
- Release-manager approval is required for any public write.

## SOTA publication controls

Each package or registry lane must define:

1. package name and registry owner/account
2. publish command and dry-run command
3. trusted-publisher/OIDC status or token fallback with expiry
4. provenance/attestation mechanism
5. SBOM/checksum artifact linkage
6. compatibility and conformance evidence
7. documentation URL
8. rollback/yank/deprecation plan
9. owner and approver

No package should be described as production-ready until these fields are complete and validated.
