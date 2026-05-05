# Maintenance Handoff

This record is completed by the release manager before a release candidate can
move to publication.

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
- Confirm every open release blocker has an owner and escalation path.

## R2 handoff status

| Area | Status | Evidence |
|---|---|---|
| Changelog policy | Ready for local static check | `docs/release/changelog-policy.md` |
| Compatibility/deprecation policy | Ready for release-manager review | `docs/release/release-governance.md`, `docs/release/compatibility.md` |
| Package publication | Blocked; dry-run only | Track 15 handoff |
| Release evidence | Required before publish | `docs/release/release-checklist.md`, `docs/release/supply-chain-verification.md` |
| Maintenance owner | Release manager plus affected surface owner | `CODEOWNERS`, `MAINTAINERS.md` |

## Follow-up queue

- Add a CI workflow that implements the changelog policy against PR diffs.
- Add release-manager sign-off to the GitHub release workflow once Track 15
  production publishing is enabled.
- Add generated release evidence links after the first dry-run candidate.
