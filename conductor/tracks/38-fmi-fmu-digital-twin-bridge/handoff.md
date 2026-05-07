# Handoff: Track 38 FMI/FMU & Digital Twin Bridge

## Summary

Established the first artifact-backed FMI/FMU and digital-twin bridge scaffold. The current implementation covers feature-gated crate boundaries, unpacked FMU layout detection, dependency-free FMU import/export layout validation, FMI 2.0 ABI function-table and safe instance wrappers, initial `modelDescription.xml` generation with dependency-free structural validation, minimal AAS JSON descriptor serialization with structural validation, validated change-detected digital-twin publications, state snapshot/diff/apply helpers, docs, and a runnable basic-import example.

## Latest hardening slice

- Import validation now rejects `modelDescription.xml` files that lack an `fmiModelDescription` root marker or FMI 2.0/3.0 version marker before checking platform binaries.
- FMI 2.0 export generation now emits output model-structure entries as `Outputs/Unknown index="..."` and validates generated XML markers/counts before writing an unpacked package.
- AAS descriptor validation now rejects duplicate submodel IDs and duplicate submodel `idShort` values in addition to nested property validation.
- Digital-twin connector validation now has fallible construction/publication paths for finite positive sample rates, finite non-negative epsilon values, valid topic prefixes, and finite values.

## Files created

`crates/kairo-ecs-fmi/Cargo.toml`, `crates/kairo-ecs-fmi/src/lib.rs`, `crates/kairo-ecs-fmi/src/error.rs`, `crates/kairo-ecs-fmi/src/import/mod.rs`, `crates/kairo-ecs-fmi/src/import/fmu_loader.rs`, `crates/kairo-ecs-fmi/src/import/fmi2.rs`, `crates/kairo-ecs-fmi/src/import/fmi3.rs`, `crates/kairo-ecs-fmi/src/import/instance.rs`, `crates/kairo-ecs-fmi/src/export/mod.rs`, `crates/kairo-ecs-fmi/src/export/model_description.rs`, `crates/kairo-ecs-fmi/src/export/packager.rs`, `crates/kairo-ecs-fmi/src/aas/mod.rs`, `crates/kairo-ecs-fmi/src/aas/descriptor.rs`, `crates/kairo-ecs-fmi/src/aas/submodel.rs`, `crates/kairo-ecs-fmi/src/digital_twin/mod.rs`, `crates/kairo-ecs-fmi/src/digital_twin/connector.rs`, `crates/kairo-ecs-fmi/src/digital_twin/sync.rs`, `docs/fmi-digital-twin/import-guide.md`, `docs/fmi-digital-twin/export-guide.md`, `docs/fmi-digital-twin/aas-mapping.md`, `docs/fmi-digital-twin/deployment-model.md`, `examples/fmi-co-simulation/README.md`, `examples/fmi-co-simulation/basic-import/Cargo.toml`, `examples/fmi-co-simulation/basic-import/src/main.rs`, `conductor/tracks/38-fmi-fmu-digital-twin-bridge/plan.md`, `conductor/tracks/38-fmi-fmu-digital-twin-bridge/test-matrix.md`, `conductor/tracks/38-fmi-fmu-digital-twin-bridge/handoff.md`

## Validation

- Passed: `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --no-default-features`
- Passed: `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --features fmi2`
- Passed: `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --features fmi3`
- Passed: `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --all-features`
- Passed: `cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --all-features --tests`
- Passed: `cargo check --manifest-path examples/fmi-co-simulation/basic-import/Cargo.toml`
- Passed: `cargo fmt --manifest-path crates/kairo-ecs-fmi/Cargo.toml`
- Blocked: `cargo test --manifest-path crates/kairo-ecs-fmi/Cargo.toml --all-features`; the Rust crate compiles, then Windows linking fails because `link.exe` resolves to `C:\Users\60217257\scoop\apps\git\current\usr\bin\link.exe` and reports `couldn't create signal pipe, Win32 error 5`.
- Blocked: `$env:RUSTFLAGS='-Clinker=rust-lld'; cargo test --manifest-path crates/kairo-ecs-fmi/Cargo.toml --all-features`; the override bypasses Git's `link.exe` but `rust-lld` cannot find Windows SDK import libraries including `kernel32.lib`, `ntdll.lib`, `userenv.lib`, `ws2_32.lib`, and `dbghelp.lib`.

## Contracts consumed

- Track 01: ECS component value serialization for FMU variable mapping and state synchronization.
- Track 02: C ABI primitives (`extern "C"` fn pointers, safe wrapping patterns) for FMU interface.
- Track 26: FMI 2.0/3.0 specification analysis, AAS metamodel review, co-simulation protocol assessment.
- Track 36: Streaming topic publication contracts for digital twin live data bridge.

## Release gates affected

- FMI features are release-critical if industrial adoption is a v1.0 goal.
- Gated behind the `fmi2` and `fmi3` Cargo feature flags; default build excludes FMI dependencies.
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
- Root workspace integration is complete: `crates/kairo-ecs-fmi` is listed in the root workspace and covered by `cargo check --workspace`.
- FMI model exchange (ME) semantics deferred to post-v1.0; co-simulation (CS) only in initial release.
- FMI 3.0 scheduled execution and clock support deferred to post-v1.0.
- FMU subprocess sandboxing for crash isolation requires platform-specific IPC; initial release may run in-process with documented risk.
- AAS schema is evolving; the connector must track AAS specification releases.
- OpenModelica CI availability: OpenModelica must be installed on CI runners for round-trip validation.
- Cross-compilation of FMU binaries (e.g., building a Linux FMU from macOS) requires documented toolchain paths.

## Worker 6 hardening evidence — 2026-05-06

- Added checked digital-twin snapshot construction and diff application via `TwinStateSnapshot::try_new()` and `try_apply()`.
- Checked synchronization now rejects empty keys, duplicate keys, removed empty keys, and diffs whose `from_tick` does not match the base snapshot tick.
- Updated `docs/fmi-digital-twin/deployment-model.md` and the test matrix to keep the current evidence bounded to local/offline validation.

## Files changed

No additional file list was recorded by this Conductor hygiene update. Use the track plan, spec, and git history for implementation-specific file evidence.


## Contracts changed

No contract changes were recorded by this Conductor hygiene update.


## Tests added

No tests were added by this Conductor hygiene update.


## Known risks

No new risks were introduced by this Conductor hygiene update.


## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.


## Integration notes

No additional integration notes were recorded by this Conductor hygiene update.
## Phase closeout evidence

Pending for the next actual phase closeout. Before this track advances, record `$conductor-review` findings, accepted fixes, deferred or blocked fixes, validation commands, cleanup state, commit SHA or explicit push blocker, and next-phase decision here.