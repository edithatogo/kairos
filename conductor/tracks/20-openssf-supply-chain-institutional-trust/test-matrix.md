# Test Matrix: Track 20 OpenSSF, Supply Chain Trust & Institutional Readiness

| Check | Validation command | Alpha | Beta | RC | 1.0 |
|---|---|---:|---:|---:|---:|
| Track-local supply-chain trust validator passes | `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/20-openssf-supply-chain-institutional-trust/validate-supply-chain-trust.ps1` | yes | yes | yes | yes |
| Track docs exist and render cleanly | `just docs-build` | yes | yes | yes | yes |
| Trust checklist names staged alpha/beta/RC/1.0 evidence | `rg -n "Release trust checklist\|Alpha\|Beta\|RC\|1.0\|SECURITY.md\|CODEOWNERS\|scorecard.yml\|dependency-review.yml\|sbom-attestations.yml\|release-attestations.yml" conductor/tracks/20-openssf-supply-chain-institutional-trust/supply-chain-plan.md` | yes | yes | yes | yes |
| Exception process records approvers, expiry, stage impact, and allowed-failure boundary | `rg -n "Temporary operational exception\|Release-stage exception\|Permanent policy waiver\|approver\|expiry\|Allowed-failure" conductor/tracks/20-openssf-supply-chain-institutional-trust/supply-chain-plan.md` | yes | yes | yes | yes |
| `conductor/quality-gates.md` includes the exact OpenSSF and supply-chain gate section | `rg -n "scorecard.yml\|dependency-review.yml\|sbom-attestations.yml\|release-attestations.yml\|waiver\|exception" conductor/quality-gates.md` | yes | yes | yes | yes |
| `conductor/delivery-readiness-checklist.md` names concrete Track 20 readiness rows | `rg -n "OpenSSF and supply-chain readiness\|scorecard.yml\|dependency-review.yml\|sbom-attestations.yml\|release-attestations.yml\|allowed-failure\|exception" conductor/delivery-readiness-checklist.md` | yes | yes | yes | yes |
| `conductor/release-engineering.md` names the trust workflows and release artifacts | `rg -n "scorecard.yml\|dependency-review.yml\|sbom-attestations.yml\|release-attestations.yml\|SBOM\|provenance\|waiver" conductor/release-engineering.md` | yes | yes | yes | yes |
| `SECURITY.md`, `CODEOWNERS`, and dependency policy are present | `Test-Path SECURITY.md; Test-Path CODEOWNERS; Test-Path renovate.json; rg -n "config:recommended\|dependencyDashboard\|vulnerabilityAlerts\|security" renovate.json` | no | yes | yes | yes |
| `.github/CODEOWNERS` is present for GitHub review enforcement | `Test-Path .github/CODEOWNERS` | yes | yes | yes | yes |
| Scorecard workflow exists | `Test-Path .github/workflows/scorecard.yml` | no | yes | yes | yes |
| Dependency review workflow exists and fails on high severity | `rg -n "fail-on-severity: high" .github/workflows/dependency-review.yml` | no | yes | yes | yes |
| SBOM and provenance workflows exist | `Test-Path .github/workflows/sbom-attestations.yml; Test-Path .github/workflows/release-attestations.yml` | no | yes | yes | yes |
| SBOM/provenance workflows request attestations and produce SPDX JSON | `rg -n "attestations: write\|actions/attest\|sbom.spdx.json\|SHA256SUMS" .github/workflows/sbom-attestations.yml .github/workflows/release-attestations.yml` | no | dry-run | yes | yes |
| SBOM attestation workflow verifies the full release artifact tree and uses a pinned artifact upload action | `rg -n "RELEASE.txt\|SHA256SUMS\|release-artifact-manifest.json\|actions/upload-artifact@[a-f0-9]{40}" .github/workflows/sbom-attestations.yml` | no | yes | yes | yes |
| SBOM/provenance plan is recorded | `rg -n "SBOM\|provenance\|attestation\|cosign" conductor/release-engineering.md conductor/quality-gates.md` | yes | yes | yes | yes |
| Vulnerability response path is documented | `rg -n "vulnerability\|security advisory\|business days\|temporary operational exceptions\|affected release stage" SECURITY.md conductor/release-engineering.md` | no | yes | yes | yes |
| Release waiver and exception process is documented | `rg -n "waiver\|exception\|override\|allowed-failure" conductor/quality-gates.md conductor/release-engineering.md conductor/tracks/20-openssf-supply-chain-institutional-trust/plan.md` | no | yes | yes | yes |
| RC artifact-tree evidence is named | `rg -n "RELEASE.txt\|SHA256SUMS\|sbom.spdx.json" conductor/quality-gates.md conductor/delivery-readiness-checklist.md conductor/tracks/20-openssf-supply-chain-institutional-trust/supply-chain-plan.md` | no | dry-run | yes | yes |
| Red-team or release-blocker escalation path is defined | `rg -n "red-team\|release blocker\|escalation" conductor/tracks/28-red-team-devils-advocate-review/spec.md conductor/release-engineering.md` | yes | yes | yes | yes |
| Aggregate Track 12-20 evidence gate keeps trust evidence wired | `node tests/conformance/track12_20_evidence_check.mjs` | yes | yes | yes | yes |

## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
