# Test Matrix — 00 Project Foundation, Governance & Naming

## Required tests

- Required Conductor setup files exist.
- All 32 track directories exist.
- Every track has `spec.md`, `plan.md`, `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md`.
- `conductor/tracks.yaml` exists as the machine-readable index.
- Initial implementation-readiness files exist.
- The setup validators succeed against the current tree.
- Track 00-06 review validator succeeds against the current track artifacts and owned implementation paths.

## CI commands

```bash
pwsh -NoProfile -File scripts/validate_conductor_setup.ps1 -SkipCargo
pwsh -NoProfile -File scripts/validate_track_coverage.ps1 -SkipCargo
test -f conductor/status.md
test -f conductor/tracks.yaml
test -f conductor/implementation-readiness.md
test -f Cargo.toml
test -d crates
test -d conformance/fixtures
test -d website
```

## Review-hardening validation

```bash
powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/00-project-foundation-governance-naming/validate-track00-06-review.ps1
```

## Done evidence validation

Track 00 may move to `Done` once live naming due-diligence evidence and maintainer approval are recorded. Recheck the evidence with:

```bash
pwsh -NoProfile -Command "Select-String -LiteralPath conductor/naming-due-diligence.md -Pattern 'Release decision','registry search date','legal/trademark advice','approver: repository maintainer'"
```

The blocker is clear when `conductor/naming-due-diligence.md` records live registry/domain/trademark/common-law search date, reviewer, query/source, exact names checked, observed result, selected names, fallback names, final public repository/module decision, and legal/trademark advice or explicit maintainer waiver for all target surfaces in the Track 00 spec.

For a `Done` status recommendation, the evidence must also be structured enough to audit:

- One live search row per exact checked name, with review date, reviewer, surface, query/source, exact name, observed result, evidence pointer, and decision impact.
- One surface decision row per target ecosystem/public identity surface, with selected public name, fallback name, public repo/module decision where applicable, legal/trademark advice or explicit maintainer waiver, release stage allowed, and approver.
- No row may rely on offline repository consistency as a substitute for live registry, domain, trademark, common-law, GitHub, Go proxy, or ecosystem search evidence.
