# Handoff: Track 20 OpenSSF, Supply Chain Trust & Institutional Readiness

## Summary

Captured the supply-chain and institutional-readiness checks that should sit alongside the release evidence pack, with the release gate tied to `SECURITY.md`, `CODEOWNERS`, `.github/workflows/scorecard.yml`, `.github/workflows/dependency-review.yml`, `.github/workflows/sbom-attestations.yml`, `.github/workflows/release-attestations.yml`, and the OpenSSF rows in the readiness and release-gate docs.

## Files changed

`conductor/tracks/20-openssf-supply-chain-institutional-trust/plan.md`, `conductor/tracks/20-openssf-supply-chain-institutional-trust/test-matrix.md`, `conductor/tracks/20-openssf-supply-chain-institutional-trust/handoff.md`

## Contracts consumed

`conductor/quality-gates.md`, `conductor/release-engineering.md`

## Release gates affected

OpenSSF Scorecard, dependency-review, SBOM, provenance, and waiver handling now feed the release gate surface before any draft release can move to publish. The beta/RC/1.0 gate remains blocked if the named workflow files or exception process are missing.

## Risks and unresolved questions

The concrete risk is a missing or incomplete GitHub Actions workflow for Scorecard, dependency review, SBOM, or provenance; keep the release gate blocked until those workflow files and the exception policy are present.
