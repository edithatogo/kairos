# Maintainer Rotation

This is the Track 16 maintainer-rotation record for the R2 release-governance
slice. It names the coverage expected before a release candidate can leave
dry-run mode. It does not enable production publishing.

Maturity: preview.

## R2 coverage map

| Role | Primary owner | Backup owner | R2 responsibility |
|---|---|---|---|
| Release manager | release-agent | governance-agent | Run the release checklist, changelog check, compatibility-policy check, and maintenance handoff. |
| Compatibility reviewer | governance-agent | api-governance-agent | Confirm changed public roots match `docs/release/compatibility.md` and `conductor/contracts/versioning-compatibility.md`. |
| Package evidence owner | release-agent | affected binding owner | Confirm Track 15 dry-run evidence, package manifest inventory, and publish blockers. |
| Supply-chain reviewer | security-agent | release-agent | Confirm SBOM, provenance, checksum, and attestation workflow evidence when generated. |
| Docs reviewer | docs-agent | release-agent | Confirm release notes, release checklist, and release docs do not overstate maturity. |

## Escalation path

1. The release manager stops the release if `compatibility-policy`,
   `changelog-check`, package dry-run, or supply-chain evidence is missing.
2. The owning track updates its handoff with the blocker, owner, and next
   validation command.
3. If a blocker must be waived, record the approver, expiry, compensating
   control, and follow-up issue before the release branch is tagged.
4. Production publishing stays disabled while Track 15 package publication is
   dry-run only.

## Rotation rule

Each release train needs one named release manager and one named backup before
RC. The same person should not be the sole approver for package evidence,
compatibility review, and supply-chain evidence on the same release.

## Local validation

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/16-release-governance-maintenance/validate-release-governance.ps1
```

Expected output includes:

```text
track16_status=ok
compatibility_policy=ok
changelog_check=ok
```
