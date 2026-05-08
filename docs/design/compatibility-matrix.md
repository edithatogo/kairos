# Compatibility Matrix

This matrix is the human-readable companion to
`docs/design/protected-surface-inventory.json`. The JSON inventory is the
machine-readable source; this file is the release-review table.

| Surface | Protected root | Current status | Breaking-change triggers | Required evidence | Release hold when |
|---|---|---|---|---|---|
| Rust types API | `crates/kairo-ecs-types` | experimental | `SimTime`, `SimDuration`, ID, shared error, or DTO semantics change | Compatibility note naming the root; ADR for public semantics; migration note before beta or later | Root missing, renamed, or semantics changed without ADR |
| Rust core API | `crates/kairo-ecs-core` | experimental | Scheduler ordering, run loop, cancellation, queue behavior, or deterministic event ordering changes | Compatibility note naming the root; ADR for public semantics; migration note before beta or later | Root missing, renamed, or semantics changed without ADR |
| Rust state API | `crates/kairo-ecs-state` | experimental | Entity/component storage, handle, query, or snapshot behavior changes | Compatibility note naming the root; ADR for public semantics; migration note before beta or later | Root missing, renamed, or semantics changed without ADR |
| Rust RNG API | `crates/kairo-ecs-rng` | experimental | Seed derivation, stream naming, distribution behavior, or deterministic output changes | Compatibility note naming the root; ADR for deterministic changes; migration note before beta or later | Root missing, renamed, or deterministic output changed without a version bump |
| C ABI | `include/kairo_ecs.h` | migration-only until public preview | Exported symbol, signature, handle ownership, allocation, status code, or error-buffer behavior changes | Compatibility note; ADR; migration note before beta or later; ABI review | Root missing, signature changed without ADR, or ownership changed without migration note |
| Arrow event-log schema | `schemas/arrow/event_log_v1.schema.json` | migration-only until telemetry preview | Field removal, field retype, field meaning change, metadata change, nullability change, or versionless schema output drift | Compatibility note; ADR for public schema changes; migration note before beta or later; schema version bump | Root missing, field removed or retyped without major schema version, or schema output changed without compatibility note |
| Python API | `bindings/python` | experimental | Import root, scheduler wrapper, Arrow helper, package API, or host behavior changes | Compatibility note naming the root; ADR for public behavior; migration note before beta or later | Root missing, renamed, or host behavior changed without migration note |
| R API | `bindings/r` | experimental | Package root, external pointer lifecycle, Arrow helper, exported function, or host behavior changes | Compatibility note naming the root; ADR for public behavior; migration note before beta or later | Root missing, renamed, or host behavior changed without migration note |
| Julia API | `bindings/julia` | experimental | Module root, `ccall` wrapper, artifact loading, Arrow integration, or host behavior changes | Compatibility note naming the root; ADR for public behavior; migration note before beta or later | Root missing, renamed, or host behavior changed without migration note |
| TypeScript/Wasm API | `bindings/typescript` | experimental | Package root, Wasm loader, browser entrypoint, Node entrypoint, typed API, or host behavior changes | Compatibility note naming the root; ADR for public behavior; migration note before beta or later | Root missing, renamed, or host behavior changed without migration note |
| C# API | `bindings/csharp` | experimental | Solution root, `SafeHandle` lifecycle, P/Invoke signature, target framework, or host behavior changes | Compatibility note naming the root; ADR for public behavior; migration note before beta or later | Root missing, renamed, or host behavior changed without migration note |
| Go API | `bindings/go` | experimental | Module root, cgo wrapper, `Close` lifecycle, exported package API, or host behavior changes | Compatibility note naming the root; ADR for public behavior; migration note before beta or later | Root missing, renamed, or host behavior changed without migration note |
| Conformance fixtures | `conformance/fixtures` | experimental | Fixture input, expected output, ordering, replay contract, or deterministic expectation drift | Compatibility note; ADR when public; fixture version bump; migration note before beta or later | Root missing or deterministic fixture output changed without version bump |

## Stage Rules

| Compatibility level | Alpha | Beta | RC | 1.0 |
|---|---|---|---|---|
| `compatible` | Allowed with compatibility note naming the affected root | Allowed with compatibility note naming the affected root | Allowed with compatibility note naming the affected root | Allowed with compatibility note naming the affected root |
| `experimental-breaking` | Allowed when the affected root is named and no published promise is contradicted | Release hold unless ADR and migration note are complete | Release hold unless ADR and migration note are complete | Release hold unless ADR and migration note are complete |
| `breaking` | Requires ADR, compatibility note, and release decision | Requires ADR, migration note, compatibility note, and release decision | Requires ADR, migration note, compatibility note, and release decision | Requires ADR, migration note, compatibility note, and release decision |
| `release-hold` | Blocks release planning until the hold reason is resolved or explicitly waived | Blocks release planning until resolved | Blocks release planning until resolved | Blocks release planning until resolved |

## Drift Checks

- The matrix must name every root from
  `docs/design/protected-surface-inventory.json`.
- Root names in this file, `conductor/contracts/versioning-compatibility.md`,
  `docs/release/compatibility.md`, `conductor/package-catalog.md`, and
  `conductor/package-matrix.md` must agree before beta or later release
  planning.
- Run `pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1` after
  editing this file.
