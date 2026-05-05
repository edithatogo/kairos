# Test Matrix — 16 Release Governance & Maintenance

## Required tests

- Unit tests.
- Integration tests if this track touches public behavior.
- Conformance fixture tests if this track touches cross-language behavior.
- Docs build if this track changes docs.
- Package dry-run if this track changes package metadata.

## CI commands

```bash
test -f conductor/release-engineering.md
test -f conductor/delivery-readiness-checklist.md
test -f conductor/quality-gates.md
test -f .github/workflows/release.yml
test -f .github/workflows/release-attestations.yml
pwsh -NoProfile -File scripts/validate_conductor_setup.ps1 -SkipCargo
```

