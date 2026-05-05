# Versioning and Compatibility Contract

## Surfaces

KairoECS versions these surfaces separately:

```text
Product/package version
Rust API version
C ABI version
Arrow schema versions
Conformance suite version
```

The current package surface inventory is:

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

Track 25 treats these surfaces as the compatibility boundary for release planning. A change is only considered reviewed when the relevant crate or package root is named in the compatibility notes.

## Breaking changes

Breaking changes include:

```text
changing event ordering semantics
changing SimTime representation without adapter
changing FFI function signatures
removing or changing Arrow fields
changing deterministic fixture outputs without version bump
changing host-language API behavior without migration guide
renaming or removing a published crate or binding package root
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

Any compatibility note, ADR, or release note must name the exact affected root and say whether the change is stable, experimental, or migration-only.

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
