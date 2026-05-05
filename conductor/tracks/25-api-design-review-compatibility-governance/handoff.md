# Handoff: Track 25 API Design Review & Compatibility Governance

## Summary

Captured the compatibility policy surface so release planning can distinguish stable, experimental, and migration-only APIs across the current Rust crates and binding package roots.

Implemented the Track 25 compatibility pack. Release planning can now distinguish
stable, experimental, and migration-only protected surfaces across Rust crates,
the C ABI header, Arrow schemas, host-language package roots, and conformance
fixtures. The policy is backed by a machine-readable inventory and a local
PowerShell validator.

## Files changed

- `conductor/contracts/versioning-compatibility.md`
- `conductor/delivery-readiness-checklist.md`
- `conductor/quality-gates.md`
- `conductor/tracks/25-api-design-review-compatibility-governance/handoff.md`
- `conductor/tracks/25-api-design-review-compatibility-governance/risk-register.md`
- `conductor/tracks/25-api-design-review-compatibility-governance/test-matrix.md`
- `docs/design/api-review.md`
- `docs/design/compatibility-governance.md`
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
rg -n "validate-compatibility-pack|protected-surface-inventory|Breaking-change rules|Release hold criteria" conductor/contracts/versioning-compatibility.md conductor/quality-gates.md conductor/delivery-readiness-checklist.md docs/design
```

Observed results:

- Protected root existence check: pass; all 13 `Test-Path` checks returned `True`.
- Policy-pack validation: pass; `compatibility pack validation passed: 13 protected surfaces`.
- Release-gate validation: pass after integration updated `docs/release/compatibility.md` to name `include/kairo_ecs.h` and `schemas/arrow/event_log_v1.schema.json`.
- Reference search: pass; policy, readiness, quality-gate, design-index, and validator references were found.
- Rust formatting check: pass; `cargo fmt --all --check` exited 0.

## Risks and unresolved questions

The main residual risk is a later API change outrunning the compatibility policy
and forcing a release hold. The policy should be consulted before any crate,
binding, ABI, Arrow schema, or conformance fixture root changes.

Another failure mode is a release note claiming compatibility while the package
catalog or matrix still points at an old root. The validator now checks the
release compatibility note in `-ReleaseGate` mode, but package catalog and
matrix drift still requires human review unless those files gain a structured
manifest in a later track.

Current blocker outside this worker's owned write scope: `docs/release/compatibility.md`
does not yet name `include/kairo_ecs.h` or `schemas/arrow/event_log_v1.schema.json`.
Release-agent follow-up is required before beta, RC, or 1.0 signoff.
