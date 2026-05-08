# Handoff: Track 25 API Design Review & Compatibility Governance

Last updated: 2026-05-08

## Summary

Captured the compatibility policy surface so release planning can distinguish stable, experimental, and migration-only APIs across the current Rust crates and binding package roots.

Implemented the Track 25 compatibility pack. Release planning can now distinguish
stable, experimental, and migration-only protected surfaces across Rust crates,
the C ABI header, Arrow schemas, host-language package roots, and conformance
fixtures. The policy is backed by a machine-readable inventory and a local
PowerShell validator.

Implementation closeout moved Track 25 to `In Review` on 2026-05-08. The
`api-review-template` and `compatibility-matrix` gates now have concrete
`docs/design` artifacts and validator coverage.

## Files changed

- `conductor/contracts/versioning-compatibility.md`
- `conductor/delivery-readiness-checklist.md`
- `conductor/quality-gates.md`
- `conductor/tracks/25-api-design-review-compatibility-governance/handoff.md`
- `conductor/tracks/25-api-design-review-compatibility-governance/risk-register.md`
- `conductor/tracks/25-api-design-review-compatibility-governance/test-matrix.md`
- `docs/design/api-review.md`
- `docs/design/api-review-template.md`
- `docs/design/compatibility-governance.md`
- `docs/design/compatibility-matrix.md`
- `docs/design/protected-surface-inventory.json`
- `docs/design/validate-compatibility-pack.ps1`

## Contracts consumed

- `conductor/delivery-readiness-checklist.md`
- `conductor/quality-gates.md`
- `conductor/contracts/versioning-compatibility.md`
- `docs/release/compatibility.md`

## Release gates affected

Compatibility review, ADR requirements, migration-note requirements, and release-hold decisions now sit on the public release path.

The release path applies to:

- `crates/kairo-ecs-types`
- `crates/kairo-ecs-core`
- `crates/kairo-ecs-state`
- `crates/kairo-ecs-rng`
- `include/kairo_ecs.h`
- `schemas/arrow/event_log_v1.schema.json`
- `bindings/python`
- `bindings/r`
- `bindings/julia`
- `bindings/typescript`
- `bindings/csharp`
- `bindings/go`
- `conformance/fixtures`

Any rename, split, merge, removal, signature change, schema change, fixture
output drift, or host API behavior change on one of those roots is breaking
unless an ADR classifies it as a compatible migration with a versioned
transition plan.

## Evidence and commands

```powershell
Test-Path -LiteralPath 'crates\kairo-ecs-types'; Test-Path -LiteralPath 'crates\kairo-ecs-core'; Test-Path -LiteralPath 'crates\kairo-ecs-state'; Test-Path -LiteralPath 'crates\kairo-ecs-rng'; Test-Path -LiteralPath 'bindings\python'; Test-Path -LiteralPath 'bindings\r'; Test-Path -LiteralPath 'bindings\julia'; Test-Path -LiteralPath 'bindings\typescript'; Test-Path -LiteralPath 'bindings\csharp'; Test-Path -LiteralPath 'bindings\go'; Test-Path -LiteralPath 'include'; Test-Path -LiteralPath 'schemas\arrow'; Test-Path -LiteralPath 'conformance\fixtures'
pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1
pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1 -ReleaseGate
node scripts/validation/validate-track21-27-evidence-boundaries.mjs
node scripts/validation/validate-tracks21-27.mjs
pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1
pwsh -NoProfile -File scripts/validate_track_no_skip_claims.ps1
rg -n "validate-compatibility-pack|api-review-template|compatibility-matrix|protected-surface-inventory|Breaking-change rules|Release hold criteria" conductor/contracts/versioning-compatibility.md conductor/quality-gates.md conductor/delivery-readiness-checklist.md docs/design conductor/api-design-review.md
```

Observed results:

- Protected root existence check: pass; all 13 `Test-Path` checks returned `True`.
- Policy-pack validation: pass; `compatibility pack validation passed: 13 protected surfaces`.
- Release-gate validation: pass; `docs/release/compatibility.md` currently names all 13 protected roots required by `docs/design/protected-surface-inventory.json`.
- Cross-track evidence-boundary validation: pass; compatibility and standards release boundaries were found.
- Phase-gate validation: pass; `0 error(s), 0 warning(s)`.
- No-skip claim validation: pass.
- Reference search: pass; policy, readiness, quality-gate, design-index, template, matrix, and validator references were found.
- Rust formatting check: pass; `cargo fmt --all --check` exited 0.
- Adjacent runner blocker: `node scripts/validation/validate-tracks21-27.mjs`
  passed Tracks 21-26 and cross-track evidence boundaries, then failed Track 27
  because docs link-check scans broken links in `bindings/typescript/node_modules`.
  This is outside Track 25 ownership.

## Risks and unresolved questions

The main residual risk is a later API change outrunning the compatibility policy
and forcing a release hold. The policy should be consulted before any crate,
binding, ABI, Arrow schema, or conformance fixture root changes.

Another failure mode is a release note claiming compatibility while the package
catalog or matrix still points at an old root. The validator checks the release
compatibility note in `-ReleaseGate` mode, but package catalog and matrix drift
still requires human review unless those files gain a structured manifest in a
later track.

Current release-gate state: `docs/release/compatibility.md` names all 13
protected roots required by `docs/design/protected-surface-inventory.json`.

## Contracts changed

`conductor/contracts/versioning-compatibility.md`, `docs/design/protected-surface-inventory.json`, and `docs/design/validate-compatibility-pack.ps1` now define the protected-surface review and release-gate contract.

## Tests added

The compatibility pack is checked with `pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1` and `pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1 -ReleaseGate`.

The same validator now also checks `docs/design/api-review-template.md` and
`docs/design/compatibility-matrix.md` for required review fields, matrix fields,
and protected-root coverage.

## Known risks

Package catalog and matrix drift can still escape the release-gate validator until those files gain structured manifest coverage.

## Follow-up issues

Add a structured package-catalog or compatibility-matrix manifest so the validator can compare package roots as well as the release compatibility note.

Resolve the adjacent Track 26 standards validator blocker before treating the
full Track 21-27 bundle as green.

## Integration notes

Any protected-root rename, split, merge, removal, signature change, schema change, fixture output drift, or host API behavior change needs ADR/versioning review before release signoff.
## Phase closeout evidence

Implementation closeout review found no in-scope defects. Track 25 is `In
Review`, not `Done`, because this local multi-worker worktree is dirty and this
slice did not commit or push.

- Review command: `$conductor-review`
- Review result: no Track 25 findings.
- Accepted fixes applied: template and matrix artifacts added; validator wired to enforce both.
- Validation passed: `pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1`; `pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1 -ReleaseGate`; `node scripts/validation/validate-track21-27-evidence-boundaries.mjs`; `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`; `pwsh -NoProfile -File scripts/validate_track_no_skip_claims.ps1`.
- Validation blocker outside Track 25: `node scripts/validation/validate-tracks21-27.mjs` fails in Track 27 because docs link-check scans broken links in `bindings/typescript/node_modules`.
- Commit SHA at validation: `b7428a311f29641ffc61cad9484301e867f32830`.
- Pushed ref: not pushed from this local multi-worker slice.
- Strict git closeout: `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` not run because this shared worktree already contains unrelated multi-worker edits.
- Next-phase decision: reviewer signoff, clean git closeout, and resolution or waiver of the Track 27 docs-link blocker are required before moving Track 25 to `Done`.
