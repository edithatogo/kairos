# Handoff — 00 Project Foundation, Governance & Naming

## Summary

Track 00 now reflects the active repository controls rather than bootstrap-era wording. The current control surface is `conductor/status.md`, `conductor/tracks.yaml`, `conductor/implementation-readiness.md`, `scripts/validate_conductor_setup.ps1`, `scripts/validate_track_coverage.ps1`, and the local Track 00 review validator for Tracks 00-06.

## Files changed

- `conductor/tracks/00-project-foundation-governance-naming/validate-track00-06-review.ps1`
- `conductor/tracks/00-project-foundation-governance-naming/handoff.md`
- `conductor/tracks/00-project-foundation-governance-naming/test-matrix.md`

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/*`, and the repo-level control docs.

## Contracts changed

`conductor/status.md`, `conductor/tracks.yaml`, and `conductor/implementation-readiness.md` are the active foundation controls.

## Tests added

- `scripts/validate_conductor_setup.ps1` and `scripts/validate_track_coverage.ps1` remain the central foundation checks this track relies on.
- `conductor/tracks/00-project-foundation-governance-naming/validate-track00-06-review.ps1` is a dependency-free local guard for this review pass. It verifies Track 00-06 required artifacts, owned implementation path presence, and absence of stale bootstrap phrases in the Track 00-06 markdown files.

## Validation run

- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/00-project-foundation-governance-naming/validate-track00-06-review.ps1`
- `pwsh -NoProfile -File scripts/validate_conductor_setup.ps1 -SkipCargo`
- `pwsh -NoProfile -File scripts/validate_track_coverage.ps1 -SkipCargo`
- Root metadata check: `codemeta.json` and `.zenodo.json` parse as JSON; `CITATION.cff`, `GOVERNANCE.md`, `LICENSE`, `LICENSE.md`, `LICENSE-MIT`, and `LICENSE-APACHE` exist.

## Closure assessment

Track 00 should remain `Spec Approved` rather than move to `Done` in this pass.

Evidence that is green:

- Governance docs exist under `governance/`, including `CODE_OF_CONDUCT.md`, `CONTRIBUTING.md`, maintainer ladder, decision-making, release-team, and security-team docs.
- ADR docs exist under `docs/adr/`, including `template.md` and recorded project-name/release-staging ADRs.
- `conductor/status.md`, `conductor/tracks.yaml`, root citation/archive metadata, and the declared Track 00 validation gates are present.
- The Track 00-06 review validator, conductor setup validator, and track coverage validator passed with cargo skipped.

Blockers to `Done`:

- `spec.md` requires `LICENSE-APACHE` and `LICENSE-MIT` files with standard full text. `LICENSE-MIT` is full text, but `LICENSE-APACHE` is an abbreviated outline rather than the standard Apache-2.0 full text.
- `spec.md` requires `naming-due-diligence.md` to contain a complete registry checklist with actual search results for all target registries. The current file lists required registries and explicitly says public publishing is blocked until registry search date, reviewer, exact names checked, search results, chosen names, fallback names, current package surfaces, and legal/trademark advice are recorded.

## Known risks

If the root metadata or GitHub repo structure changes, the validators need to be kept in sync so the foundation does not drift.

## Integration notes

Next implementation step: lock in the repository identity and naming rules against the real root metadata, then keep the validators as the gate for future foundation changes.
