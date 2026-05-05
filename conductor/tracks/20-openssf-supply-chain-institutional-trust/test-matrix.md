# Test Matrix: Track 20 OpenSSF, Supply Chain Trust & Institutional Readiness

| Check | Validation command | Alpha | Beta | RC | 1.0 |
|---|---|---:|---:|---:|---:|
| Track docs exist and render cleanly | `just docs-build` | yes | yes | yes | yes |
| `conductor/quality-gates.md` includes the exact OpenSSF and supply-chain gate section | `rg -n "scorecard.yml|dependency-review.yml|sbom-attestations.yml|release-attestations.yml|waiver|exception" conductor/quality-gates.md` | yes | yes | yes | yes |
| `conductor/release-engineering.md` names the trust workflows and release artifacts | `rg -n "scorecard.yml|dependency-review.yml|sbom-attestations.yml|release-attestations.yml|SBOM|provenance|waiver" conductor/release-engineering.md` | yes | yes | yes | yes |
| `SECURITY.md`, `CODEOWNERS`, and dependency policy are present | `Test-Path SECURITY.md; Test-Path CODEOWNERS; Test-Path renovate.json` | no | yes | yes | yes |
| Scorecard workflow exists | `Test-Path .github/workflows/scorecard.yml` | no | yes | yes | yes |
| Dependency review workflow exists and fails on high severity | `rg -n "fail-on-severity: high" .github/workflows/dependency-review.yml` | no | yes | yes | yes |
| SBOM and provenance workflows exist | `Test-Path .github/workflows/sbom-attestations.yml; Test-Path .github/workflows/release-attestations.yml` | no | yes | yes | yes |
| SBOM/provenance plan is recorded | `rg -n "SBOM|provenance|attestation|cosign" conductor/release-engineering.md conductor/quality-gates.md` | yes | yes | yes | yes |
| Vulnerability response path is documented | `rg -n "vulnerability|security advisory|response" SECURITY.md conductor/release-engineering.md` | no | yes | yes | yes |
| Release waiver and exception process is documented | `rg -n "waiver|exception|override|allowed-failure" conductor/quality-gates.md conductor/release-engineering.md conductor/tracks/20-openssf-supply-chain-institutional-trust/plan.md` | no | yes | yes | yes |
| Red-team or release-blocker escalation path is defined | `rg -n "red-team|release blocker|escalation" conductor/tracks/28-red-team-devils-advocate-review/spec.md conductor/release-engineering.md` | yes | yes | yes | yes |
