# API Design Review Template

Complete this review before a public API change is treated as beta-ready or stable.

## Affected roots

- `crates/kairo-ecs-types`
- `crates/kairo-ecs-core`
- `crates/kairo-ecs-state`
- `crates/kairo-ecs-rng`
- `bindings/python`
- `bindings/r`
- `bindings/julia`
- `bindings/typescript`
- `bindings/csharp`
- `bindings/go`

## Review form

```markdown
# API Review: <name>

## Problem

## Affected root

## Surface family

## Proposed API

### Rust
### C ABI
### Python
### R
### Julia
### TypeScript
### C#
### Go

## Compatibility matrix

| Surface | Current root | Change type | Breaking? | Stable / preview / migration-only | Notes |
|---|---|---|---|---|---|
| Rust API | | | | | |
| C ABI | | | | | |
| Arrow schema | | | | | |
| Host-language API | | | | | |

## Breaking-change policy

- Changing event ordering semantics is breaking.
- Changing `SimTime` representation without an adapter is breaking.
- Changing FFI function signatures is breaking.
- Removing or changing Arrow fields is breaking.
- Changing deterministic fixture outputs without a version bump is breaking.
- Changing host-language API behavior without a migration guide is breaking.
- Renaming or removing a published crate or binding package root is breaking.

## Migration notes

- Required before beta, RC, or 1.0 when consumer code changes.
- Must name the exact affected root.
- Must state whether the transition is stable, preview, or migration-only.
- Must say what downstream code needs to change.

## Memory ownership

## Error model

## Thread-safety model

## Determinism/replay impact

## Conformance fixtures added

## Alternatives rejected

## Red-team objections

## Decision

- [ ] accepted
- [ ] rejected
- [ ] needs prototype
- [ ] needs ADR
```

## Notes

- If the review cannot name the exact affected root, it is incomplete.
- If the compatibility matrix conflicts with `conductor/contracts/versioning-compatibility.md`, update the policy before release planning continues.
- If a change alters a published root name, release planning should treat it as a hold until the transition is documented.

