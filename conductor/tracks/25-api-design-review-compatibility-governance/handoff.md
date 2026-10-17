# Handoff: Track 25 API Design Review & Compatibility Governance

Last updated: 2026-05-09

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
- Focused Track 21-27 validation: pass; Track 25 compatibility policy pack
  and cross-track evidence boundaries passed with the adjacent docs workflow.

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
- Focused Track 21-27 validation: `node scripts/validation/validate-tracks21-27.mjs` passed, including Track 25 compatibility policy pack and cross-track evidence boundaries.
- Commit SHA at validation: `7111f227446bbd0c24d24c636c4a052141bcec7f`.
- Pushed ref: not pushed from this local multi-worker slice.
- Strict git closeout: `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` failed with `Working tree has uncommitted tracked or untracked changes; closeout requires a clean tree`.
- Next-phase decision: Track 25 has no compatibility-pack validation blocker. Reviewer signoff, commit/push evidence, and strict clean-worktree closeout are still required before moving Track 25 to `Done`.

## CareOps Q0.1 Flow design review intake — 2026-09-29

The Kairos repository owner directed adoption of the CareOps Q0.1 recommended
architecture. Its Flow-specific adapter introduces public Rust surfaces in both
`crates/kairo-ecs-des` and `crates/kairo-ecs-abm`; both exact roots are recorded
as experimental in the protected-surface inventory and aligned policy, matrix,
and release compatibility note. The design review is at
`docs/design/api-reviews/flow-runtime-q0.1.md`.

This intake accepts only the architecture and surface classification. Concrete
Flow symbols still require Q0.2/Q0.3 contracts and symbol-level review before
implementation is released. The new API remains under release hold. This note
does not close Track 25 or replace its remaining phase gates.


## C1 shared temporal helper prerequisite (2026-10-04)

Approved internal experimental direction adds KnowledgeAvailable to a public enum; exhaustive matches require migration. Release hold remains; no universal nonbreaking or external-owner claim. See `../../design/calibration/c1-shared-temporal-helper-v1.md`. This is a scoped development extension; historical closeout evidence remains unchanged.

## Q5.3 Flow compatibility handoff — 2026-10-05

The Q5.3 source-bound compatibility/migration qualification is at Kairos `eae890b0a2a3524a543ec4ee4aca61346e273b52` on `origin/codex/careops-q53-compatibility` (stacked/draft development PR #218); see the CareOps Sim parent `conductor/evidence/q5.3-completion-20261005/`. The external legacy consumer fixture and source-linked migration guide preserve the legacy `Resource`/`DESContext` surface within the tested cases; they are not a general semver guarantee. Flow remains experimental. The existing Track 25 Q0.1 note remains applicable: concrete Flow symbols still require Track 25 review before release. No symbol-level review, stable promotion, formal release-gate run, release approval, or security-gate waiver is claimed here; historical Track 25 status and plan remain unchanged.
