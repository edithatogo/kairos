# Handoff — 00 Project Foundation, Governance & Naming

## Summary

Track 00 now reflects the active repository controls rather than bootstrap-era wording. The current control surface is `conductor/status.md`, `conductor/tracks.yaml`, `conductor/implementation-readiness.md`, `scripts/validate_conductor_setup.ps1`, `scripts/validate_track_coverage.ps1`, and the local Track 00 review validator for Tracks 00-06. Naming due-diligence evidence and the Track 00 release decision are checked in at `conductor/naming-due-diligence.md`, with supporting live-search notes in `conductor/naming-due-diligence-live-evidence.md`.

## Files changed

- `conductor/tracks/00-project-foundation-governance-naming/validate-track00-06-review.ps1`
- `conductor/tracks/00-project-foundation-governance-naming/handoff.md`
- `conductor/tracks/00-project-foundation-governance-naming/test-matrix.md`
- `conductor/naming-due-diligence-live-evidence.md`
- `conductor/naming-due-diligence.md`

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
- `pwsh -NoProfile -Command "Select-String -LiteralPath LICENSE-APACHE -Pattern '9. Accepting Warranty or Additional Liability','APPENDIX: How to apply the Apache License to your work.'"`
- Root metadata check: `codemeta.json` and `.zenodo.json` parse as JSON; `CITATION.cff`, `GOVERNANCE.md`, `LICENSE`, `LICENSE.md`, `LICENSE-MIT`, and `LICENSE-APACHE` exist.

## Closure assessment

Track 00 can move to `Done` in this pass after repository maintainer approval of the naming decision on 2026-05-07.

Evidence that is green:

- Governance docs exist under `governance/`, including `CODE_OF_CONDUCT.md`, `CONTRIBUTING.md`, maintainer ladder, decision-making, release-team, and security-team docs.
- ADR docs exist under `docs/adr/`, including `template.md` and recorded project-name/release-staging ADRs.
- `conductor/status.md`, `conductor/tracks.yaml`, root citation/archive metadata, and the declared Track 00 validation gates are present.
- `LICENSE-APACHE` now carries the standard Apache License 2.0 full text, and `LICENSE-MIT` already carries the standard MIT License full text.
- The Track 00-06 review validator, conductor setup validator, and track coverage validator passed with cargo skipped.
- Live registry and public-identity search rows now exist for the main naming surfaces.
- The repository maintainer approved the naming evidence and accepted an explicit legal/trademark waiver for Track 00 closeout on 2026-05-07.

Blockers to `Done`:

- None remaining for Track 00. Later release tracks still own production publishing, registry release, domain use, SBOM, provenance, and any refreshed legal/trademark review required before RC/1.0.

## Known risks

If the root metadata or GitHub repo structure changes, the validators need to be kept in sync so the foundation does not drift.

## Integration notes

Next implementation step: lock in the repository identity and naming rules against the real root metadata, then keep the validators as the gate for future foundation changes.

## Naming due-diligence evidence update — 2026-05-07

`conductor/naming-due-diligence.md` now records both the dated offline evidence pass and a live registry/public-identity pass for the current checked-in package surfaces. The evidence remains conservative: local repository sources, plus live search rows and a maintainer waiver for Track 00 closeout, without claiming formal legal clearance.

Local evidence captured:

- Project identity is locally consistent as `KairoECS` across README, citation/archive metadata, and ADR 0004.
- Current metadata repository URL is locally consistent as `https://github.com/edithatogo/kairos`.
- Checked-in manifests declare `kairo-ecs-*` Rust crate names, Python distribution `kairo-ecs` with import package `kairo_ecs`, R package `kairoECS`, Julia package `KairoECS`, npm package `@kairo-ecs/typescript`, NuGet package `Kairo.ECS`, and Go module `github.com/edithatogo/kairos/bindings/go`.

The live pass recorded exact-name search rows for crates.io, PyPI, npm, NuGet, Julia registry, R channels, Go module discovery, GitHub repo discovery, domains, and trademark/common-law review. The pass still does not fabricate legal clearance or registrar ownership.

Track 00 naming acceptance is approved by the repository maintainer as of 2026-05-07. The precise follow-up checklist is recorded in `conductor/naming-due-diligence.md` and requires refreshed repo/module/domain decisions plus formal legal/trademark advice before RC/1.0 public publishing.

Naming Worker 6 integration readiness check: the Done evidence structure is now explicit. The required structure is one live search row per exact checked name and one surface decision row per target ecosystem/public identity surface. Offline local-manifest consistency is useful context, but it is not sufficient to mark Track 00 `Done` without the live registry, domain, trademark, common-law, GitHub/module, ecosystem evidence rows, and approver/legal-waiver outcome fields now recorded in `conductor/naming-due-diligence.md`.

## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.
