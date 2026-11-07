# Release Compatibility and Deprecation Notes

This register stays aligned with the
[versioning contract](../../conductor/contracts/versioning-compatibility.md),
[capability register](../../conductor/capabilities.json) and
[release gates](../../conductor/release-gates.md).

## Current public roots

| Surface | Roots | Current boundary |
| --- | --- | --- |
| Rust crates | `crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-des`, `crates/kairo-ecs-state`, `crates/kairo-ecs-rng` and optional crates | Alpha APIs; each capability needs its own acceptance evidence. |
| C ABI | `crates/kairo-ecs-ffi`, `include/kairo_ecs.h` | Versioned, ownership-sensitive native contract; installed libraries and adapters must match. |
| Host adapters | `bindings/python`, `bindings/r`, `bindings/julia`, `bindings/typescript`, `bindings/csharp`, `bindings/go` | Actual Rust native/Wasm execution only. Unsupported operations fail; imports and metadata are not runtime qualification. |
| Arrow schemas | `schemas/arrow/`, `schemas/arrow/event_log_v1.schema.json` | Semantic changes require versioning and compatibility notes. |
| Conformance fixtures | `conformance/fixtures/` | Output drift requires a compatibility note, explicit review and version policy. |
| Release artefacts | manifests, checksums, SBOM/provenance and archive metadata | The alpha source release exists; package-registry and installed-consumer qualification remain separately gated. |

## Compatibility posture

- The current alpha is not a stable 1.0 compatibility promise.
- Public roots must be named in changelog and release notes when changed.
- Breaking API, ABI, schema, fixture, package or release-classification changes
  require an ADR and migration instructions.
- Historical receipts remain attributed to their original revision and runtime.
  A supported capability must pass current acceptance for the new release.

## Deprecation and removal register

| Affected root | Removed support | Replacement | Notice and effect | Decision |
| --- | --- | --- | --- | --- |
| Host-language scheduler/model facades in Python, R, TypeScript, C# and Go; Julia engine-availability claims | Independent host-language simulation cores and synthetic simulation fallback | A qualified adapter to Rust native code or Rust-compiled Wasm; direct Rust APIs where an adapter is incomplete | 7 November 2026, immediate alpha breaking change explicitly requested by the project owner; no gradual migration or dual-support period | [ADR 0010](../adr/0010-rust-only-execution.md), [migration](migration.md), [Track 109](../../conductor/tracks/109-rust-only-runtime/plan.md) |

Retained API names may raise an explicit unsupported-operation error. That is a
migration aid, not continued support for the old engine. This removal does not
change the meaning of previously captured test results; it prevents them from
being presented as Rust-native qualification.
