# Test Matrix — 16 Release Governance & Maintenance

## Required tests

- Unit tests.
- Integration tests if this track touches public behavior.
- Conformance fixture tests if this track touches cross-language behavior.
- Docs build if this track changes docs.
- Package dry-run if this track changes package metadata.
- Changelog policy static check for public release-surface changes.
- Changelog policy workflow presence and PR diff enforcement.
- Release governance evidence check for compatibility, deprecation,
  maintenance handoff docs, and the implemented release workflow path.
- Aggregate Track 12-20 evidence check for release, citation, benchmark, and
  supply-chain gate wiring.

## CI commands

```bash
test -f conductor/release-engineering.md
test -f conductor/delivery-readiness-checklist.md
test -f conductor/quality-gates.md
test -f .github/workflows/changelog-policy.yml
test -f .github/workflows/release.yml
test -f .github/workflows/release-attestations.yml
pwsh -NoProfile -File scripts/validate_conductor_setup.ps1 -SkipCargo
powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/16-release-governance-maintenance/validate-release-governance.ps1
node tests/conformance/track12_20_evidence_check.mjs
Select-String -Path .github/workflows/release.yml -Pattern 'Validate release checklist|Build release manifest|Validate release manifest|cargo publish --dry-run --workspace|release workflow is dry-run only'
Select-String -Path .github/workflows/changelog-policy.yml -Pattern 'Public release surface changed without CHANGELOG.md|changelog_policy=ok'
Select-String -Path docs/release/changelog-policy.md -Pattern 'Public release surface changed without CHANGELOG.md'
Select-String -Path docs/release/release-governance.md -Pattern 'Compatibility gate|dry-run'
```

## R2 governance slice checks

```powershell
Test-Path CHANGELOG.md
Test-Path docs/release/release-governance.md
Test-Path docs/release/changelog-policy.md
Test-Path docs/release/compatibility.md
Test-Path docs/release/maintenance-handoff.md
Test-Path docs/release/maintainer-rotation.md
Test-Path .github/workflows/changelog-policy.yml
Test-Path .github/workflows/release.yml
Select-String -Path CHANGELOG.md -Pattern 'Release governance slice'
Select-String -Path CHANGELOG.md -Pattern 'maintainer rotation'
Select-String -Path conductor/tracks.yaml -Pattern 'compatibility-policy|changelog-check'
Select-String -Path conductor/quality-gates.md -Pattern '\*\*compatibility-policy\*\*|\*\*changelog-check\*\*'
Select-String -Path docs/release/release-governance.md -Pattern 'Compatibility gate'
Select-String -Path docs/release/changelog-policy.md -Pattern 'Public release surface changed without CHANGELOG.md'
Select-String -Path docs/release/compatibility.md -Pattern 'Deprecation register'
Select-String -Path docs/release/maintenance-handoff.md -Pattern 'R2 handoff status'
Select-String -Path docs/release/maintainer-rotation.md -Pattern 'Maturity: preview|Release manager|Escalation path'
Select-String -Path .github/workflows/changelog-policy.yml -Pattern 'Public release surface changed without CHANGELOG.md|changelog_policy=ok'
Select-String -Path .github/workflows/release.yml -Pattern 'Validate release checklist|Build release manifest|Validate release manifest|cargo publish --dry-run --workspace|release workflow is dry-run only'
powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/16-release-governance-maintenance/validate-release-governance.ps1
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

## 2026-05-09 implementation refresh evidence

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/16-release-governance-maintenance/validate-release-governance.ps1
node tests/conformance/track12_20_evidence_check.mjs
pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1
```

The refresh advances Track 16 to `In Review` because the Track 16 validator and
global phase-gate validator pass locally and the earlier Track 19 phase-gate
blocker is no longer present.

## 2026-05-09 review evidence

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/16-release-governance-maintenance/validate-release-governance.ps1
node tests/conformance/track12_20_evidence_check.mjs
pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1
```

All three commands passed in the review pass.

The local changelog-policy diff check against the shared worktree currently
fails because `bindings/julia/src/KairoECS.jl` and
`bindings/julia/test/runtests.jl` are modified without a matching
`CHANGELOG.md` diff. That is a cross-track public-surface blocker outside
Track 16 ownership; the Track 16 release-governance validator itself remains
green.
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
