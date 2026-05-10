# API Design Review and Compatibility Governance

Every public API addition must complete this review before reaching a stable host-language package or a beta-or-later release claim.

Use `docs/design/api-review-template.md` to open a review record and
`docs/design/compatibility-matrix.md` to classify release-stage consequences.
The JSON inventory in `docs/design/protected-surface-inventory.json` remains the
machine-readable root list.

## Current protected surfaces

| Surface family | Current checked-in roots | Review consequence |
|---|---|---|
| Rust crate APIs | `crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, `crates/kairo-ecs-rng` | Name the exact crate root in the compatibility note, ADR, or migration note. |
| Host-language package APIs | `bindings/python`, `bindings/r`, `bindings/julia`, `bindings/typescript`, `bindings/csharp`, `bindings/go` | Name the exact binding root in the compatibility note, ADR, or migration note. |
| C ABI surface | `include/kairo_ecs.h` | Treat signature, symbol, ownership, allocation, status-code, and error-buffer changes as release-gated. |
| Arrow telemetry schemas | `schemas/arrow/event_log_v1.schema.json` | Treat field, metadata, nullability, meaning, and schema-version changes as compatibility changes even when the Rust API is unchanged. |
| Scenario / conformance surfaces | `conformance/fixtures` | Version fixture outputs and note any change that alters deterministic expectations. |

## Compatibility matrix

| Surface | Current release posture | Breaking change trigger | Required evidence |
|---|---|---|---|
| Rust crate APIs | Checked-in workspace roots; release-gated | Public API, semver, or deterministic behavior changes | Compatibility note naming the exact crate root, plus ADR if public behavior changes |
| Host-language package APIs | Checked-in package skeletons; release-gated | Root rename, API shape change, ownership change, or behavior drift | Compatibility note naming the exact package root, plus migration note before beta or later |
| C ABI | Policy surface only until Track 02 lands | Signature, ownership, or symbol changes | ADR, migration note, and ABI review before any release claim |
| Arrow schemas | Policy surface only until schema release work lands | Field removal, meaning change, or versionless output drift | Schema note, ADR if public, and migration note before beta or later |
| Scenario / conformance surfaces | Checked-in shared contract and fixtures | Fixture output, ordering, or replay contract drift | Fixture/version update plus compatibility note naming the affected fixture or scenario |

## Breaking changes

Breaking changes include:

- changing event ordering semantics
- changing `SimTime` representation without an adapter
- changing FFI function signatures
- removing or changing Arrow fields
- changing deterministic fixture outputs without a version bump
- changing host-language API behavior without a migration guide
- renaming or removing a published crate or binding package root

If a change matches any item above, it is breaking unless an ADR explicitly classifies it as a compatible migration with a versioned transition plan.

## Review rules

### ADR required

An ADR is required before merge when a change:

- alters public API, ABI, or schema semantics
- renames, splits, merges, or removes a published crate or binding package root
- changes the release-stage classification of a surface
- changes the compatibility promise for a stable or experimental surface

### Migration note required

A migration note is required before beta, RC, or 1.0 when a change:

- changes user-facing behavior in a way that could break existing scripts, examples, or downstream bindings
- requires adapter code, version pinning, or source edits in consumers
- introduces a new root name that replaces an old root name

The migration note must name the exact affected root, say what changes for consumers, and state whether the transition is stable, preview, or migration-only.

### Release hold criteria

Release is held until the compatibility review is complete when:

- the affected crate or package root is not named in the review notes
- the compatibility policy conflicts with the package catalog or package matrix
- a breaking change is proposed without an ADR
- a breaking change is proposed for beta, RC, or 1.0 without a migration note
- the change alters a published root name without a documented transition plan

### Package-root alignment

The live roots that must stay aligned across policy, catalog, and release notes are:

```text
crates/kairo-ecs-types
crates/kairo-ecs-core
crates/kairo-ecs-state
crates/kairo-ecs-rng
bindings/python
bindings/r
bindings/julia
bindings/typescript
bindings/csharp
bindings/go
```

Any compatibility note, ADR, or release note must name the exact affected root and say whether the change is stable, experimental, preview, or migration-only.

## Review template fields

Every API review must use `docs/design/api-review-template.md` and capture:

```text
affected_root
surface_family
compatibility_level
breaking_change? yes/no
migration_note_required? yes/no
adr_required? yes/no
release_hold? yes/no
consumer_impact
deprecation_or_transition_plan
fixture_or_schema_impact
red_team_objections
decision
```

## Compatibility files

Each release must update:

```text
CHANGELOG.md
docs/release/compatibility.md
docs/release/migration.md if needed
conductor/contracts/versioning-compatibility.md if policy changes
conductor/package-catalog.md if package roots changed
conductor/package-matrix.md if package availability changed
```

## Enforcement summary

- If the change is breaking, require ADR and migration note.
- If the change renames a root, require a release hold until the transition is documented.
- If the compatibility note cannot name the exact root, it is incomplete.
- If the package catalog or package matrix is stale, update those before release planning proceeds.
