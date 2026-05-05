# Versioning and Compatibility Contract

## Protected surface inventory

KairoECS versions these surfaces separately:

```text
Product/package version
Rust API version
C ABI version
Arrow schema versions
Conformance suite version
```

The machine-readable inventory lives in:

```text
docs/design/protected-surface-inventory.json
```

Track 25 treats these roots as the compatibility boundary for release planning.
A change is only considered reviewed when the relevant root is named in the
compatibility notes, ADR, migration note, and release-hold decision when those
artifacts are required.

| Surface | Root | Current status | Concrete protected example |
|---|---|---|---|
| Rust types API | `crates/kairo-ecs-types` | experimental | `SimTime`, `SimDuration`, IDs, shared errors, and versioned DTOs |
| Rust core API | `crates/kairo-ecs-core` | experimental | Scheduler ordering, run loop, queue behavior, cancellation, and deterministic event ordering |
| Rust state API | `crates/kairo-ecs-state` | experimental | Entity/component storage, generational handles, query behavior, and state snapshots |
| Rust RNG API | `crates/kairo-ecs-rng` | experimental | Seed derivation, stream naming, and deterministic distribution behavior |
| C ABI | `include/kairo_ecs.h` | migration-only until public preview | `kairo_ecs_ffi_version`, handle ownership, status codes, and exported C symbols |
| Arrow event log schema | `schemas/arrow/event_log_v1.schema.json` | migration-only until telemetry preview | Event-log fields, metadata keys, nullability, and schema version |
| Python API | `bindings/python` | experimental | Python import root, scheduler wrapper, Arrow helpers, and package-level API |
| R API | `bindings/r` | experimental | R package root, external pointer lifecycle, Arrow helpers, and exported functions |
| Julia API | `bindings/julia` | experimental | Julia module root, `ccall` wrappers, artifact loading, and Arrow integration |
| TypeScript/Wasm API | `bindings/typescript` | experimental | TypeScript package root, Wasm loader, browser/Node entrypoints, and typed API |
| C# API | `bindings/csharp` | experimental | C# solution root, `SafeHandle` lifecycle, P/Invoke signatures, and target framework API |
| Go API | `bindings/go` | experimental | Go module root, cgo wrapper, `Close` lifecycle, and exported package API |
| Conformance fixtures | `conformance/fixtures` | experimental | Shared deterministic fixture inputs, expected outputs, ordering, and replay contracts |

## Breaking-change rules

Breaking changes include:

```text
changing event ordering semantics
changing SimTime representation without adapter
changing FFI function signatures
changing C ABI handle ownership, allocation, or error-buffer semantics
removing or changing Arrow fields
changing Arrow field meaning, nullability, metadata, or version without schema policy
changing deterministic fixture outputs without version bump
changing host-language API behavior without migration guide
renaming or removing a published crate or binding package root
splitting or merging a published crate or binding package root
changing package import/module names or default entrypoints
weakening a stable, preview, or experimental compatibility promise
```

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

The migration note must name the exact affected root, explain what downstream
users must change, identify the first affected release stage, and state whether
the transition is stable, experimental, or migration-only.

### Release hold criteria

Release is held until the compatibility review is complete when:

- the affected root is not named in the review notes
- the compatibility policy conflicts with the package catalog or package matrix
- a breaking change is proposed without an ADR
- a breaking change is proposed for beta, RC, or 1.0 without a migration note
- the change alters a published root name without a documented transition plan
- the machine-readable inventory is missing the affected root
- `docs/design/validate-compatibility-pack.ps1 -ReleaseGate` fails

### Package-root alignment

The live roots that must stay aligned across policy, catalog, and release notes
are the roots in `docs/design/protected-surface-inventory.json`.

Any compatibility note, ADR, or release note must name the exact affected root
and say whether the change is stable, experimental, or migration-only.

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
- Run `pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1` after any edit to this contract or the protected-surface inventory.
- Run `pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1 -ReleaseGate` before beta, RC, or 1.0 release signoff and after any edit to `docs/release/compatibility.md`.
