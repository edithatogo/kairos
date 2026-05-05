# Handoff — 02 The Bridge: kairo-ecs-ffi, UniFFI & Diplomat

## Summary

Track 02 now defines the bridge contract between the Rust core and the generated language bindings. The current control surface is the bridge spec, bridge plan, bridge test matrix, the shared core contracts in tracks 01 and 12, and the concrete binding crate roots under `crates/kairo-ecs-ffi`, `crates/kairo-ecs-uniffi`, `crates/kairo-ecs-diplomat`, and `include/`.

## Files changed

No code files were changed in this handoff pass.

## Contracts consumed

`crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, `crates/kairo-ecs-rng`, and the shared Track 12 fixture manifest.

## Contracts changed

Binding-facing handles, status codes, generated API surfaces, and the Rust facade boundary that the wrappers must not outgrow.

## Tests added

The bridge test matrix now uses guarded workspace commands and explicit repo-gate checks; generated binding builds will be added when the package manifests exist.

## Known risks

The main risk is ABI drift between the Rust facade and generated bindings if the core types move before Track 12 fixtures and the bridge wrappers are updated.

## Integration notes

Next implementation step: land the pure Rust bridge facade first, then expose UniFFI and Diplomat outputs against that frozen surface while keeping Track 12 parity checks in step.
