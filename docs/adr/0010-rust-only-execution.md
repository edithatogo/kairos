# ADR 0010: Rust is the only supported simulation core

## Status

Accepted, 7 November 2026. This decision implements the project owner's explicit
instruction to end support for non-Rust simulation cores immediately. There is
no gradual migration or parallel support period.

## Context

Earlier language packages offered independent preview schedulers, random streams
or demonstration model execution. Passing their unit tests could be mistaken for
qualification of the Rust engine. Multiple implementations also multiply the
work required to preserve scheduling, state, RNG and numerical semantics.

The repository has a substantial Rust implementation and a versioned C ABI and
Wasm bridge. Those are the execution boundaries to improve and qualify.

## Decision

1. Rust owns event scheduling, simulation state, random streams and model
   execution. Rust-compiled WebAssembly is an allowed target of the same core.
2. Python, R, Julia, TypeScript, C# and Go may prepare inputs, orchestrate
   experiments and analyse outputs. Their supported engine APIs must invoke
   the actual Rust native library or a generated Rust/Wasm module.
3. A missing library, incompatible ABI, absent symbol, unavailable backend or
   unimplemented operation must fail explicitly. No fallback may execute an
   alternative host-language core or return synthetic simulation results.
4. Existing host-only scheduler and simulation facades are unsupported
   immediately. Retained names may raise a migration error; they do not offer
   a compatibility execution path.
5. Adapter qualification requires an installed consumer to load the built
   engine and exercise the promised operations. Package imports, metadata,
   DTO tests and file-existence checks remain useful but prove a narrower scope.
6. Applications, including `careops-sim`, own their domain policies and models
   outside Kairos. They consume a pinned Kairos dependency through versioned
   public Rust APIs, the ABI or generated schemas. Kairos does not depend on
   the application's repository layout or internal implementation.

## Consequences

This is a breaking alpha change. Users of preview host engines must use a
qualified Rust adapter or move simulation execution to Rust. An unavailable
adapter is reported as unsupported until it passes its native acceptance gates;
this decision does not assert that all language adapters are already complete.

The native C ABI currently exposes a smaller surface than the Rust API. The
Python `NativeEngine` and TypeScript generated-Wasm loader are documented only
for operations they actually forward. Vectorized RL, arbitrary model callbacks,
time-travel restoration and other broader interfaces need separate native
implementations and acceptance evidence before support is advertised.

Legacy outputs and benchmark receipts remain historical evidence with their
original implementation labels. Host-only results cannot qualify a new Rust
adapter. Performance comparisons must measure the actual execution path.

## Completion and release gates

- [Native-only policy](../../conductor/native-only-policy.md)
- [Track 109: Rust-only runtime](../../conductor/tracks/109-rust-only-runtime/plan.md)
- [Capability register](../../conductor/capabilities.json)
- [Beta and stable release gates](../../conductor/release-gates.md)
- [Migration guide](../release/migration.md)
- [Compatibility register](../release/compatibility.md)

This supersedes any earlier documentation allowing a pure-language preview core
as a supported fallback. The existing C ABI ownership, error and versioning
contracts continue to apply to native adapters.
