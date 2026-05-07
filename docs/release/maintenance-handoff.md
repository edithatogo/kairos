# Maintenance Handoff

This record is completed by the release manager before a release candidate can
move to publication.

Current status: release governance is documented and locally checkable, but
publication remains blocked while Track 15 stays in dry-run mode and registry
name/toolchain verification remains unverified on the target machines.

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
| Package publication | Blocked; dry-run only until Track 15 clears registry/name/toolchain evidence | Track 15 handoff |
| Release evidence | Required before publish | `docs/release/release-checklist.md`, `docs/release/supply-chain-verification.md` |
| Registry/toolchain verification | Blocked pending target-machine checks and registry name verification | Track 15 handoff |
| Maintenance owner | Release manager plus affected surface owner | `CODEOWNERS`, `MAINTAINERS.md` |

## Follow-up queue

- Add release-manager sign-off to the GitHub release workflow once Track 15
  production publishing is enabled.
- Add generated release evidence links after the first dry-run candidate.
- Record the first successful dry-run candidate with the artifact manifest,
  checksum manifest, and blocker note before any publish gate is cleared.
