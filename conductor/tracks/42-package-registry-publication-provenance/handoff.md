# 42 Package Registry Publication & Provenance - handoff.md

Last updated: 2026-06-25

## Summary

Track 42 implements the guarded package registry publication lane. The current slice adds the manifest, validator, workflow, and release-gate wiring. Public publication remains blocked until registry accounts/trusted publishers are configured and the protected environment approves a run.

Archive review on 2026-06-25 found no remaining source/workflow defect in the repo-side guarded publication gate. Track 42 is Done only for the repo-side manifest, validator, dry-run helper, protected workflow, OIDC/provenance, and code-health gating surfaces; live public registry writes remain blocked by external release operations and evidence requirements.

## Files changed

- `.github/workflows/registry-publish.yml`
- `packaging/publication-registry-manifest.json`
- `scripts/release/publish-registry.mjs`
- `scripts/validation/validate-publication-readiness.mjs`
- Conductor registry and quality-gate files

## Contracts consumed

- Track 15 package dry-run inventory
- Track 20 supply-chain trust controls
- Track 44 code/repo health floor

## Contracts changed

Public language registry writes now require Track 42 and Track 44 gates.

## Tests added

- `node scripts/validation/validate-publication-readiness.mjs`

## Known risks

Trusted publisher setup is registry-side and cannot be completed by repo edits alone.

The first protected-environment GitHub Actions dispatch and any live public registry publication evidence are intentionally not claimed by this archive.

## Follow-up issues

- Configure PyPI/TestPyPI trusted publishers.
- Configure npm trusted publisher or provenance-capable token path.
- Configure NuGet trusted publishing or scoped API key fallback.
- Configure crates.io owner/token/trusted-publishing path.
- Decide R-universe/CRAN and Julia registry submission timing.

## Integration notes

Use the protected `release-publication` environment for public writes. Dry runs can run without registry credentials.

## Phase closeout evidence

2026-06-25 review/validation commands:

- `node scripts/validation/validate-publication-readiness.mjs`
- `node scripts/validation/validate-code-health.mjs`
- `python packaging/scripts/build_release_manifest.py --verify-existing`
- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1`

Archive commit SHA and pushed ref are recorded in `conductor/phase-closeout.yaml`. Run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after the archive commit is pushed.
