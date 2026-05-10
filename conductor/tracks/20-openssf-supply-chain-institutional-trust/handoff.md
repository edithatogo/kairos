# Handoff: Track 20 OpenSSF, Supply Chain Trust & Institutional Readiness

Last updated: 2026-05-09

## Summary

Captured the supply-chain and institutional-readiness checks that should sit alongside the release evidence pack, with the release gate tied to `SECURITY.md`, `CODEOWNERS`, `.github/CODEOWNERS`, `renovate.json`, `.github/workflows/scorecard.yml`, `.github/workflows/dependency-review.yml`, `.github/workflows/actions-security.yml`, `.github/workflows/workflow-security.yml`, `.github/workflows/secret-scan.yml`, `.github/workflows/sbom-attestations.yml`, `.github/workflows/release-attestations.yml`, and the OpenSSF rows in the readiness and release-gate docs.

This pass added the concrete Track 20 trust checklist, exception categories, machine-check references, and RC artifact-tree requirements for `RELEASE.txt`, `SHA256SUMS`, and `sbom.spdx.json`.

The 2026-05-08 local R2 evidence pass generated `dist/release-artifact-manifest.json` and `dist/SHA256SUMS` through the Track 15 dry-run builder. SBOM and provenance evidence remain blocked locally because `syft` is not installed in this shell and the GitHub hosted attestation workflows are failing before job steps start. Do not claim SBOM, provenance, or attestation evidence until either the GitHub workflows run successfully or a local SBOM tool is installed and the generated `dist/sbom.spdx.json` is validated.

## Files changed

`conductor/tracks/20-openssf-supply-chain-institutional-trust/supply-chain-plan.md`, `conductor/tracks/20-openssf-supply-chain-institutional-trust/test-matrix.md`, `conductor/tracks/20-openssf-supply-chain-institutional-trust/risk-register.md`, `conductor/tracks/20-openssf-supply-chain-institutional-trust/handoff.md`, `conductor/delivery-readiness-checklist.md`, `conductor/quality-gates.md`

## Contracts consumed

`conductor/quality-gates.md`, `conductor/release-engineering.md`, `.github/workflows/scorecard.yml`, `.github/workflows/dependency-review.yml`, `.github/workflows/sbom-attestations.yml`, `.github/workflows/release-attestations.yml`

## Release gates affected

OpenSSF Scorecard, dependency-review, SBOM, provenance, and waiver handling now feed the release gate surface before any draft release can move to publish. Workflow hardening, secret scanning, and artifact-tree checks sit alongside that release gate. The beta/RC/1.0 gate remains blocked if the named workflow files or exception process are missing. RC and 1.0 are blocked if the release artifact tree lacks `RELEASE.txt`, `SHA256SUMS`, or `sbom.spdx.json`.

## Risks and unresolved questions

The concrete risk is a missing or incomplete GitHub Actions workflow for Scorecard, dependency review, SBOM, provenance, or secret scanning, or an exception record that lacks approvers, expiry, or stage impact. Keep the release gate blocked until the artifact-tree checks pass or an approved exception is recorded under `supply-chain-plan.md`.

Current blocker: artifact manifest and checksum evidence exist locally for R2 dry-run, but SBOM/provenance evidence does not. Hosted GitHub Actions jobs currently fail with no executed steps, so the trust evidence cannot be promoted beyond offline dry-run documentation.

## Validation evidence

- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/20-openssf-supply-chain-institutional-trust/validate-supply-chain-trust.ps1`
- `rg -n "Release trust checklist|Temporary operational exception|Permanent policy waiver|RELEASE.txt|SHA256SUMS|sbom.spdx.json" conductor/tracks/20-openssf-supply-chain-institutional-trust/supply-chain-plan.md`
- `rg -n "OpenSSF and supply-chain readiness|scorecard.yml|dependency-review.yml|sbom-attestations.yml|release-attestations.yml|allowed-failure|exception" conductor/delivery-readiness-checklist.md`
- `rg -n "Machine-checkable release-trust references|fail-on-severity|actions/attest|sbom.spdx.json|SHA256SUMS|Exception review" conductor/quality-gates.md`
- `Test-Path SECURITY.md; Test-Path CODEOWNERS; Test-Path .github/CODEOWNERS; Test-Path .github/workflows/scorecard.yml; Test-Path .github/workflows/dependency-review.yml; Test-Path .github/workflows/sbom-attestations.yml; Test-Path .github/workflows/release-attestations.yml`
- `pwsh -NoProfile -File scripts/validate_track_docs_clean.ps1`
- `npm run build` from `website/`
- `git diff --check -- conductor/tracks/20-openssf-supply-chain-institutional-trust/supply-chain-plan.md conductor/tracks/20-openssf-supply-chain-institutional-trust/plan.md conductor/tracks/20-openssf-supply-chain-institutional-trust/test-matrix.md conductor/tracks/20-openssf-supply-chain-institutional-trust/risk-register.md conductor/tracks/20-openssf-supply-chain-institutional-trust/handoff.md conductor/delivery-readiness-checklist.md conductor/quality-gates.md`
- `Get-Command syft -ErrorAction SilentlyContinue` returned no command in this shell on 2026-05-08.

## Review-hardening update

Added a track-local offline validator for release-trust evidence and softened
the audit language in `spec.md` so Track 20 does not imply an audit report
exists before it is checked in.

## Implementation-review update

`$conductor-implement` and `$conductor-review` were run for the Track 20-owned surface on 2026-05-08. The implementation pass hardened `SECURITY.md` with vulnerability response and exception expectations, tightened `.github/workflows/sbom-attestations.yml` so SBOM attestation verifies `RELEASE.txt`, `release-artifact-manifest.json`, and `SHA256SUMS`, and updated the Track 20 validator/test matrix plus global readiness gate references.

Accepted fixes: SBOM attestation checkout now disables persisted credentials, the SBOM artifact upload uses `actions/upload-artifact@v4`, and the local Track 20 gate asserts the vulnerability-policy text and full artifact-tree checks.

Review findings: no blocking defect remains inside the Track 20-owned files after the accepted fixes.

Recorded blockers:

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` fails because Tracks 15 and 27 are `In Review` without phase closeout ledger entries, outside Track 20 ownership.
- `dist/RELEASE.txt` and `dist/sbom.spdx.json` are absent in the local artifact tree, so RC/1.0 SBOM/provenance claims remain blocked until the release tree is regenerated with those files.
- `syft`, `actionlint`, and `zizmor` are not installed in this shell, so local SBOM generation and local workflow-security lint execution were not available.
- `.github/workflows/release-attestations.yml` is outside this Track 20 ownership pass and still needs release/CI owner review before release-stage attestation claims are promoted.

Track status: In Review. The scorecard, sbom-plan, and vulnerability-policy gates are satisfied for repository evidence and beta-stage planning; RC/1.0 artifact evidence remains blocked as listed above.

## Contracts changed

No supply-chain contracts changed in this scoped cleanup; trust evidence remains tied to checked-in security docs, ownership files, and workflow definitions.

## Tests added

No executable tests were added in this scoped cleanup. Existing evidence remains `validate-supply-chain-trust.ps1`.

## Known risks

Supply-chain trust claims can drift if workflow behavior changes without matching readiness and release-gate documentation updates.

## Follow-up issues

Keep Scorecard, dependency review, secret scanning, workflow security, SBOM, and attestation evidence aligned before any release-trust claim is promoted.

## Integration notes

Treat Track 20 as a trust gate and evidence map, not as proof of an external security audit unless a real audit artifact is checked in.

## Renovate migration review update

Reviewed Track 20 after the Renovate migration on 2026-05-09. The offline
trust validator now parses `renovate.json` and requires the recommended preset,
dependency dashboard preset, explicit dependency dashboard enablement,
vulnerability alerts, and the `security` label on vulnerability-alert PRs. The
readiness checklist, quality gates, supply-chain plan, and test matrix now name
that dependency-policy evidence instead of treating the presence of
`renovate.json` alone as sufficient.

The SBOM attestation evidence rows were also aligned with the current workflow
hardening posture: the Track 20 gate now expects a pinned
`actions/upload-artifact` action hash in `.github/workflows/sbom-attestations.yml`,
matching the validator and the checked-in workflow.

Validation run on 2026-05-09:

- PASS: `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\20-openssf-supply-chain-institutional-trust\validate-supply-chain-trust.ps1`
- PASS: `rg -n "config:recommended|dependencyDashboard|vulnerabilityAlerts|security" renovate.json`
- PASS: `rg -n "RELEASE.txt|SHA256SUMS|release-artifact-manifest.json|actions/upload-artifact@[a-f0-9]{40}" .github\workflows\sbom-attestations.yml`
- PASS: `node tests\conformance\track12_20_evidence_check.mjs`
- PASS: `pwsh -NoProfile -File scripts\validate_track_docs_clean.ps1`
- PASS: `pwsh -NoProfile -File scripts\validate_conductor_phase_gates.ps1`
- PASS: `pwsh -NoProfile -File scripts\validate_conductor_git_closeout.ps1`
- PASS with Git line-ending warning only: `git diff --check -- SECURITY.md .github\workflows\scorecard.yml .github\workflows\sbom-attestations.yml conductor\delivery-readiness-checklist.md conductor\quality-gates.md conductor\tracks\20-openssf-supply-chain-institutional-trust`
- FAIL expected until commit/cleanup: `pwsh -NoProfile -File scripts\validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` reported uncommitted tracked or untracked changes.
- BLOCKED locally: `syft`, `actionlint`, and `zizmor` are not installed in this shell.
- BLOCKED for RC/1.0 artifact evidence: `dist\RELEASE.txt` and `dist\sbom.spdx.json` are absent; `dist\SHA256SUMS` and `dist\release-artifact-manifest.json` are present.

Done eligibility after this review: Track 20 repository-evidence gates are
green for alpha/beta planning. Track 20 is not Done for RC/1.0 release-trust
claims until the local worktree is clean or committed, missing release artifact
evidence is generated or explicitly excepted, and unavailable local SBOM/workflow
lint tools are either installed or covered by hosted CI evidence.
## Phase closeout evidence

`$conductor-review` completed for the Track 20-owned surface on 2026-05-08. Accepted fixes are listed above. Deferred or blocked fixes are limited to Track 15/27 phase-gate ledger cleanup, RC/1.0 artifact generation, missing local workflow/SBOM tools, and non-owned release attestation review. Commit SHA: blocked because this pass has not been committed or pushed. Pushed ref: blocked because no push was attempted. `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` was not run because the worktree intentionally contains uncommitted Track 20 closeout edits. Next-phase decision: Track 20 is In Review; do not advance to Done until RC/1.0 artifact evidence or explicit release-stage exceptions are available and shared phase-gate blockers are cleared.
