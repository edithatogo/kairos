# Compatibility Governance

Track 25 uses `conductor/contracts/versioning-compatibility.md` as the policy
source of truth and `docs/design/protected-surface-inventory.json` as the
machine-readable inventory. `docs/design/api-review-template.md` is the intake
artifact for the `api-review-template` gate, and
`docs/design/compatibility-matrix.md` is the release-review artifact for the
`compatibility-matrix` gate.

## Protected Surface Inventory

The inventory names each root that must be mentioned in compatibility notes,
ADRs, migration notes, or release-hold decisions:

- Rust crate APIs: `crates/kairo-ecs-types`, `crates/kairo-ecs-core`,
  `crates/kairo-ecs-state`, and `crates/kairo-ecs-rng`.
- C ABI: `include/kairo_ecs.h`.
- Arrow schemas: `schemas/arrow/event_log_v1.schema.json`.
- Host APIs: `bindings/python`, `bindings/r`, `bindings/julia`,
  `bindings/typescript`, `bindings/csharp`, and `bindings/go`.
- Conformance fixtures: `conformance/fixtures`.

## Review Outcome Rules

Every protected-surface change is classified as one of:

- `compatible`: additive or clarifying; allowed at any stage with a compatibility
  note naming the affected root.
- `experimental-breaking`: allowed before beta only when the affected root is
  named and the change does not contradict a published compatibility promise.
- `breaking`: requires an ADR, a migration note before beta or later, and a
  release-stage decision.
- `release-hold`: blocks beta, RC, and 1.0 until policy, release notes, package
  roots, ADRs, and migration notes agree.

Run the local validator after editing compatibility policy or release notes:

```powershell
pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1
pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1 -ReleaseGate
```

The non-release validator checks that the template and matrix exist, name every
protected root, and retain the release-hold and evidence fields needed by
reviewers.
