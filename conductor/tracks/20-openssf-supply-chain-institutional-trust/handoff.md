# Handoff: Track 20 OpenSSF, Supply Chain Trust & Institutional Readiness

## Summary

Captured the supply-chain and institutional-readiness checks that should sit alongside the release evidence pack, with the release gate tied to `SECURITY.md`, `CODEOWNERS`, `.github/CODEOWNERS`, `.github/dependabot.yml` or `renovate.json`, `.github/workflows/scorecard.yml`, `.github/workflows/dependency-review.yml`, `.github/workflows/actions-security.yml`, `.github/workflows/workflow-security.yml`, `.github/workflows/secret-scan.yml`, `.github/workflows/sbom-attestations.yml`, `.github/workflows/release-attestations.yml`, and the OpenSSF rows in the readiness and release-gate docs.

This pass added the concrete Track 20 trust checklist, exception categories, machine-check references, and RC artifact-tree requirements for `RELEASE.txt`, `SHA256SUMS`, and `sbom.spdx.json`.

## Files changed

`conductor/tracks/20-openssf-supply-chain-institutional-trust/supply-chain-plan.md`, `conductor/tracks/20-openssf-supply-chain-institutional-trust/test-matrix.md`, `conductor/tracks/20-openssf-supply-chain-institutional-trust/risk-register.md`, `conductor/tracks/20-openssf-supply-chain-institutional-trust/handoff.md`, `conductor/delivery-readiness-checklist.md`, `conductor/quality-gates.md`

## Contracts consumed

`conductor/quality-gates.md`, `conductor/release-engineering.md`, `.github/workflows/scorecard.yml`, `.github/workflows/dependency-review.yml`, `.github/workflows/sbom-attestations.yml`, `.github/workflows/release-attestations.yml`

## Release gates affected

OpenSSF Scorecard, dependency-review, SBOM, provenance, and waiver handling now feed the release gate surface before any draft release can move to publish. Workflow hardening, secret scanning, and artifact-tree checks sit alongside that release gate. The beta/RC/1.0 gate remains blocked if the named workflow files or exception process are missing. RC and 1.0 are blocked if the release artifact tree lacks `RELEASE.txt`, `SHA256SUMS`, or `sbom.spdx.json`.

## Risks and unresolved questions

The concrete risk is a missing or incomplete GitHub Actions workflow for Scorecard, dependency review, SBOM, provenance, or secret scanning, or an exception record that lacks approvers, expiry, or stage impact. Keep the release gate blocked until the artifact-tree checks pass or an approved exception is recorded under `supply-chain-plan.md`.

## Validation evidence

- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/20-openssf-supply-chain-institutional-trust/validate-supply-chain-trust.ps1`
- `rg -n "Release trust checklist|Temporary operational exception|Permanent policy waiver|RELEASE.txt|SHA256SUMS|sbom.spdx.json" conductor/tracks/20-openssf-supply-chain-institutional-trust/supply-chain-plan.md`
- `rg -n "OpenSSF and supply-chain readiness|scorecard.yml|dependency-review.yml|sbom-attestations.yml|release-attestations.yml|allowed-failure|exception" conductor/delivery-readiness-checklist.md`
- `rg -n "Machine-checkable release-trust references|fail-on-severity|actions/attest|sbom.spdx.json|SHA256SUMS|Exception review" conductor/quality-gates.md`
- `Test-Path SECURITY.md; Test-Path CODEOWNERS; Test-Path .github/CODEOWNERS; Test-Path .github/workflows/scorecard.yml; Test-Path .github/workflows/dependency-review.yml; Test-Path .github/workflows/sbom-attestations.yml; Test-Path .github/workflows/release-attestations.yml`
- `pwsh -NoProfile -File scripts/validate_track_docs_clean.ps1`
- `npm run build` from `website/`
- `git diff --check -- conductor/tracks/20-openssf-supply-chain-institutional-trust/supply-chain-plan.md conductor/tracks/20-openssf-supply-chain-institutional-trust/plan.md conductor/tracks/20-openssf-supply-chain-institutional-trust/test-matrix.md conductor/tracks/20-openssf-supply-chain-institutional-trust/risk-register.md conductor/tracks/20-openssf-supply-chain-institutional-trust/handoff.md conductor/delivery-readiness-checklist.md conductor/quality-gates.md`

## Review-hardening update

Added a track-local offline validator for release-trust evidence and softened
the audit language in `spec.md` so Track 20 does not imply an audit report
exists before it is checked in.
