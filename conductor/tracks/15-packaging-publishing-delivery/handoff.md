# Handoff — 15 Packaging, Publishing & Delivery

Last updated: 2026-05-09

## Summary

Early-stage registry and packaging plan updated for Rust, Python, R, Julia, TypeScript, C#, and Go.
The track remains in dry-run mode and does not publish packages to any registry.

R2 dry-run evidence now has a reusable package manifest inventory and checksum
builder under `packaging/`, with the release workflow consuming the same local
script used by developers for both evidence generation and generated-evidence
verification.

Worker 6 defined the first local registry/package dry-run sequence in
`packaging/release-package-manifest.json`. The sequence is offline-only:
inventory check, local evidence generation, and the Track 15 validator. It
explicitly disallows publish manifests before later release gates enable them.

The local R2 dry-run evidence set was generated on 2026-05-08:

- `dist/release-artifact-manifest.json` generated with version `0.0.0-r2-dry-run`.
- `dist/SHA256SUMS` generated for the same package-manifest inventory.
- The generated manifest covers 32 package manifests across Rust, Python, R, Julia, TypeScript, C#, and Go.
- `python packaging/scripts/build_release_manifest.py --verify-existing` now verifies generated `dist/` evidence against the package inventory and checksums.
- `dist/` remains ignored; this handoff records the durable evidence state while generated artifacts stay local/reproducible.
- The release workflow now has a Track 15 release-delivery gate before artifact upload, and the new validator reports the current blocker state when SBOM/provenance evidence is absent.

## Files changed

`conductor/tracks/15-packaging-publishing-delivery/spec.md`
`conductor/tracks/15-packaging-publishing-delivery/plan.md`
`conductor/tracks/15-packaging-publishing-delivery/test-matrix.md`
`conductor/tracks/15-packaging-publishing-delivery/agent-contract.md`
`conductor/tracks/15-packaging-publishing-delivery/risk-register.md`
`conductor/tracks/15-packaging-publishing-delivery/handoff.md`
`packaging/README.md`
`packaging/release-package-manifest.json`
`packaging/scripts/build_release_manifest.py`
`.github/workflows/release.yml`
`docs/release/release-checklist.md`
`docs/release/supply-chain-verification.md`
`scripts/validate_track15_release_delivery.ps1`

## Contracts consumed

`conductor/workflow.md`
`conductor/contracts/*` where present
`conductor/package-matrix.md`
`conductor/package-catalog.md`

## Contracts changed

None.

## Tests added

Local manifest/checksum validation:

```text
python packaging/scripts/build_release_manifest.py --check
python packaging/scripts/build_release_manifest.py --version 0.0.0-r2-dry-run
powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/15-packaging-publishing-delivery/validate-packaging-dry-run.ps1
```

2026-05-08 local evidence validation:

```text
python packaging/scripts/build_release_manifest.py --version 0.0.0-r2-dry-run
powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/15-packaging-publishing-delivery/validate-packaging-dry-run.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/validate_track15_release_delivery.ps1
```

Result: `validated 32 package manifests across 7 ecosystems`; `track15_status=ok`; release-delivery gate reports the current blocker before artifact upload when SBOM/provenance evidence is absent.

## Known risks

Registry name availability remains unverified.
Toolchain installs and registry availability must be verified on the target machines before any production publish step is enabled.
The local builder verifies package manifest presence and checksums only; it does
not execute ecosystem pack commands or contact registries.
The first local sequence also rejects network-required steps, credentials,
uploads, publish commands, and publish manifest files.

## Integration notes

Keep release work in draft and dry-run mode until name reservations, toolchain support, and registry policies are verified.

## Review-hardening update

Added a track-local dry-run validator that checks the package manifest
structure and fails if the track drifts toward production publishing before
the release gates are met.

Worker 6 extended the validator to cover the ordered local dry-run sequence and
to fail if publish/publication manifest files appear under `packaging/` or
`dist/`.

## Follow-up issues

Keep public publishing blocked until registry names, legal metadata, compatibility gates, and dry-run package evidence are complete across the package matrix.
## Phase closeout evidence

2026-05-09 implementation slice:

- Added `python packaging/scripts/build_release_manifest.py --verify-existing` so generated `dist/release-artifact-manifest.json` and `dist/SHA256SUMS` are verified against `packaging/release-package-manifest.json` before release workflow artifact upload.
- Wired `.github/workflows/release.yml` to use the shared verifier instead of inline manifest logic.
- Updated the Track 15 release-delivery validator and release docs to require generated-evidence verification.
- Validation passed:
  - `python packaging/scripts/build_release_manifest.py --check`
  - `python packaging/scripts/build_release_manifest.py --version 0.0.0-r2-dry-run`
  - `python packaging/scripts/build_release_manifest.py --verify-existing`
  - `pwsh -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/15-packaging-publishing-delivery/validate-packaging-dry-run.ps1`
  - `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_track15_release_delivery.ps1`
  - `node tests/conformance/track12_20_evidence_check.mjs`
- Formal `$conductor-review`, commit/push closeout, and strict clean-tree git closeout remain pending coordinator review.

Before the next phase advances, record `$conductor-review` findings, accepted fixes, deferred or blocked fixes, validation commands, cleanup state, commit SHA or explicit push blocker, pushed ref, strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` result, and next-phase decision here.
