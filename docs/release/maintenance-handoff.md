# Maintenance Handoff

This record is completed by the release manager before a release candidate can
move to publication.

Current status: release governance is documented and locally checkable, but
publication remains blocked while Track 15 stays in dry-run mode and registry
name/toolchain verification remains unverified on the target machines.

## Actual package archive rehearsal — 2026-10-01

The package dry-run workflow now builds archive files, gathers them into a
checksummed tree, and retains that tree as a GitHub Actions artifact for 90 days.
The local rehearsal at source `738f2206f7a59e338c169d771faae7b73cd6372b`
produced eight files: one Rust `.crate`, Python wheel and source distribution,
R source package, npm `.tgz`, NuGet `.nupkg`, and Julia/Go source archives.
Julia and Go source archives are fallbacks for their untagged registry surfaces;
they are not registry package artifacts. R CMD check completed with one NOTE for
missing optional suggested packages and skipped package tests per the workflow
command. Full Rust workspace packaging remains blocked by internal path
dependencies without registry versions.

The local archive index, build receipts, checksum list, and ZIP are retained in
the caller's ignored `.artifacts/` directory. After the workflow change is
merged, the combined `kairos-actual-package-archives-<commit>` Actions artifact
will be the shared 90-day copy. Neither copy enables registry publication.

Latest local dry-run evidence, generated on 2026-05-08:

- `dist/release-artifact-manifest.json` was generated with version `0.0.0-r2-dry-run`.
- `dist/SHA256SUMS` was generated for the same package-manifest inventory.
- `python packaging/scripts/build_release_manifest.py --verify-existing` verifies that both generated files still match `packaging/release-package-manifest.json`.
- The generated manifest reports 32 package manifests across Rust, Python, R, Julia, TypeScript, C#, and Go.
- `dist/` is ignored and remains a reproducible local evidence output, not a tracked release artifact.
- Production publishing remains disabled and blocked pending registry/name/toolchain verification.

## Release manager checklist

- Confirm `CHANGELOG.md` contains the release entry and any deprecations.
- Confirm `docs/release/release-notes.md` matches the changelog and does not
  add unsupported compatibility claims.
- Confirm `docs/release/compatibility.md` names every changed public root.
- Confirm breaking changes have an ADR and migration note where required.
- Confirm Track 15 package evidence remains dry-run unless publication gates
  are explicitly cleared.
- Confirm artifact manifest, checksum, SBOM, and provenance evidence paths are
  recorded when generated.
- Confirm the release blocker state is recorded: registry name availability
  remains unverified, target-machine toolchains remain unverified, and
  production publish stays disabled.
- Confirm every open release blocker has an owner and escalation path.

## R2 handoff status

| Area | Status | Evidence |
|---|---|---|
| Changelog policy | Ready for local static check | `docs/release/changelog-policy.md` |
| Changelog policy workflow | Implemented | `.github/workflows/changelog-policy.yml`, `docs/release/changelog-policy.md` |
| Compatibility/deprecation policy | Ready for release-manager review | `docs/release/release-governance.md`, `docs/release/compatibility.md` |
| Maintainer rotation | Ready for RC assignment; production publish still blocked | `docs/release/maintainer-rotation.md` |
| Package publication | Blocked; dry-run only until Track 15 clears registry/name/toolchain evidence | Track 15 handoff |
| Release evidence | Local R2 dry-run evidence generated; publish evidence still blocked | `dist/release-artifact-manifest.json`, `dist/SHA256SUMS`, Track 15 handoff |
| Registry/toolchain verification | Blocked pending target-machine checks and registry name verification | Track 15 handoff |
| Maintenance owner | Release manager plus affected surface owner | `CODEOWNERS`, `MAINTAINERS.md` |

## Follow-up queue

- Add release-manager sign-off to the GitHub release workflow once Track 15
  production publishing is enabled.
- Add generated release evidence links after the first dry-run candidate.
- Record the first successful dry-run candidate with the artifact manifest,
  checksum manifest, and blocker note before any publish gate is cleared.
- Assign concrete human release-manager and backup names before RC.
