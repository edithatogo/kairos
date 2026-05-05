# Handoff: Track 38 FMI/FMU & Digital Twin Bridge

## Summary

Established the first artifact-backed FMI/FMU and digital-twin bridge scaffold. The current implementation covers feature-gated crate boundaries, unpacked FMU layout detection, FMI 2.0 ABI function-table and safe instance wrappers, initial `modelDescription.xml` generation, minimal AAS JSON descriptor serialization, change-detected digital-twin publications, state snapshot/diff/apply helpers, docs, and a runnable basic-import example.

## Files created

`crates/kairo-ecs-fmi/Cargo.toml`, `crates/kairo-ecs-fmi/src/lib.rs`, `crates/kairo-ecs-fmi/src/error.rs`, `crates/kairo-ecs-fmi/src/import/mod.rs`, `crates/kairo-ecs-fmi/src/import/fmu_loader.rs`, `crates/kairo-ecs-fmi/src/import/fmi2.rs`, `crates/kairo-ecs-fmi/src/import/fmi3.rs`, `crates/kairo-ecs-fmi/src/import/instance.rs`, `crates/kairo-ecs-fmi/src/export/mod.rs`, `crates/kairo-ecs-fmi/src/export/model_description.rs`, `crates/kairo-ecs-fmi/src/export/packager.rs`, `crates/kairo-ecs-fmi/src/aas/mod.rs`, `crates/kairo-ecs-fmi/src/aas/descriptor.rs`, `crates/kairo-ecs-fmi/src/aas/submodel.rs`, `crates/kairo-ecs-fmi/src/digital_twin/mod.rs`, `crates/kairo-ecs-fmi/src/digital_twin/connector.rs`, `crates/kairo-ecs-fmi/src/digital_twin/sync.rs`, `docs/fmi-digital-twin/import-guide.md`, `docs/fmi-digital-twin/export-guide.md`, `docs/fmi-digital-twin/aas-mapping.md`, `docs/fmi-digital-twin/deployment-model.md`, `examples/fmi-co-simulation/README.md`, `examples/fmi-co-simulation/basic-import/Cargo.toml`, `examples/fmi-co-simulation/basic-import/src/main.rs`, `conductor/tracks/38-fmi-fmu-digital-twin-bridge/plan.md`, `conductor/tracks/38-fmi-fmu-digital-twin-bridge/test-matrix.md`, `conductor/tracks/38-fmi-fmu-digital-twin-bridge/handoff.md`

## Validation

- Passed: `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --no-default-features`
- Passed: `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --features fmi2`
- Passed: `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --all-features`
- Passed: `cargo check --manifest-path examples/fmi-co-simulation/basic-import/Cargo.toml`
- Blocked: `cargo test --manifest-path crates/kairo-ecs-fmi/Cargo.toml --features fmi2`; the Rust crate compiles, then Windows linking fails because `link.exe` resolves to `C:\Users\60217257\scoop\apps\git\current\usr\bin\link.exe` and reports `couldn't create signal pipe, Win32 error 5`.

## Contracts consumed

- Track 01: ECS component value serialization for FMU variable mapping and state synchronization.
- Track 02: C ABI primitives (`extern "C"` fn pointers, safe wrapping patterns) for FMU interface.
- Track 26: FMI 2.0/3.0 specification analysis, AAS metamodel review, co-simulation protocol assessment.
- Track 36: Streaming topic publication contracts for digital twin live data bridge.

## Release gates affected

- FMI features are release-critical if industrial adoption is a v1.0 goal.
- Gated behind `fmi` Cargo feature flag; default build excludes FMI dependencies.
- FMU export binary reproducibility is a release gate (identical model → identical FMU).
- AAS descriptor is published alongside release artifacts.

## FMI version support matrix

| FMI mode | Current status |
|---|---|
| FMI 2.0 co-simulation import | ABI table and safe wrapper scaffold present; dynamic loading and real FMU execution pending |
| FMI 2.0 export | Initial `modelDescription.xml` and unpacked package layout writer present; binary packaging pending |
| FMI 2.0 model exchange | Deferred |
| FMI 3.0 co-simulation import/export | Feature flag and reserved module present; implementation deferred |
| FMI 3.0 clocks/scheduled execution | Deferred |

## Platform compatibility table

| Platform | Current status |
|---|---|
| Windows x86_64 | Layout detection maps to `binaries/win64`; cargo check passes; cargo test blocked by local linker resolution |
| Linux x86_64 | Layout detection maps to `binaries/linux64`; CI validation still required |
| macOS x86_64 | Layout detection maps to `binaries/darwin64`; CI validation still required |
| macOS arm64 | Layout detection maps to `binaries/darwinaarch64`; CI validation still required |

## Risks and unresolved questions

- `.fmu` archive extraction and dynamic symbol loading are blocked until dependency policy accepts archive and dynamic-loading crates such as `zip` and `libloading`, or an equivalent internal implementation is provided.
- Root workspace integration is blocked for this worker because Track 38 ownership did not include root `Cargo.toml`; the crate is currently an isolated package with its own `[workspace]`.
- FMI model exchange (ME) semantics deferred to post-v1.0; co-simulation (CS) only in initial release.
- FMI 3.0 scheduled execution and clock support deferred to post-v1.0.
- FMU subprocess sandboxing for crash isolation requires platform-specific IPC; initial release may run in-process with documented risk.
- AAS schema is evolving; the connector must track AAS specification releases.
- OpenModelica CI availability: OpenModelica must be installed on CI runners for round-trip validation.
- Cross-compilation of FMU binaries (e.g., building a Linux FMU from macOS) requires documented toolchain paths.
