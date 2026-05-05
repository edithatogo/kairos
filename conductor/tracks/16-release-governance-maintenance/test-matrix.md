# Test Matrix — 16 Release Governance & Maintenance

## Required tests

- Unit tests.
- Integration tests if this track touches public behavior.
- Conformance fixture tests if this track touches cross-language behavior.
- Docs build if this track changes docs.
- Package dry-run if this track changes package metadata.
- Changelog policy static check for public release-surface changes.
- Release governance evidence check for compatibility, deprecation, and
  maintenance handoff docs.

## CI commands

```bash
test -f conductor/release-engineering.md
test -f conductor/delivery-readiness-checklist.md
test -f conductor/quality-gates.md
test -f .github/workflows/release.yml
test -f .github/workflows/release-attestations.yml
pwsh -NoProfile -File scripts/validate_conductor_setup.ps1 -SkipCargo
```

## R2 governance slice checks

```powershell
Test-Path CHANGELOG.md
Test-Path docs/release/release-governance.md
Test-Path docs/release/changelog-policy.md
Test-Path docs/release/compatibility.md
Test-Path docs/release/maintenance-handoff.md
Select-String -Path CHANGELOG.md -Pattern 'Release governance slice'
Select-String -Path docs/release/release-governance.md -Pattern 'Compatibility gate'
Select-String -Path docs/release/changelog-policy.md -Pattern 'Public release surface changed without CHANGELOG.md'
Select-String -Path docs/release/compatibility.md -Pattern 'Deprecation register'
Select-String -Path docs/release/maintenance-handoff.md -Pattern 'R2 handoff status'
```

## CI gate definition

The release workflow should block publish when:

- the PR changes a public release surface without `CHANGELOG.md`;
- a breaking change lacks an ADR;
- a removal lacks a deprecation-register row or ADR;
- release notes claim compatibility not present in
  `docs/release/compatibility.md`;
- Track 15 package evidence is missing or still marked blocked while a publish
  job is requested.

