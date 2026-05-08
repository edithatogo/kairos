# Track 38 Plan: FMI/FMU & Digital Twin Bridge

## Phase 0 — Contract alignment

### Task 0.1 — Read existing contracts
- Review FMI 2.0.4 and FMI 3.0 specification from Track 26 standards review artifacts.
- Review C ABI contracts from Track 02: function pointer types, safe wrapper patterns, error handling conventions.
- Review streaming data pipeline contracts from Track 36: topic schema, serialization format, connection lifecycle.
- Review core state contracts from Track 01: how ECS component values are serialized and restored.
- Open an ADR on the FMI version strategy: FMI 2.0 only, FMI 3.0 only, or dual support.

### Task 0.2 — Define owned artifacts
- Define `kairo-ecs-fmi` crate boundaries: import runtime, export tooling, AAS connector, digital twin connector.
- Define `FmuInstance` trait contract: life-cycle methods (`instantiate`, `enter_initialization_mode`, `exit_initialization_mode`, `do_step`, `terminate`, `free_instance`).
- Define export pipeline: `modelDescription.xml` template, source stub generation, CMake or cargo-based FMU packaging.
- Define AAS descriptor v3.0 schema compliance targets.
- Add owner/subagent to `conductor/subagents.md`.

## Phase 1 — FMI 2.0 import runtime

### Task 1.1 — Scaffold kairo-ecs-fmi crate
- Create `crates/kairo-ecs-fmi/` with Cargo feature flags: `fmi2`, `fmi3`, `aas`, `digital-twin`.
- Implement FMU shared library loader: unzip FMU archive, locate platform-specific `.so`/`.dylib`/`.dll`, load via `libloading`.
- Implement FMI 2.0 C function pointer table: `fmi2Instantiate`, `fmi2SetupExperiment`, `fmi2EnterInitializationMode`, `fmi2ExitInitializationMode`, `fmi2DoStep`, `fmi2Terminate`, `fmi2FreeInstance`, `fmi2GetReal`, `fmi2SetReal`, and scalar accessors.

Current artifacts:
- `crates/kairo-ecs-fmi/` exists as a root workspace crate with `fmi2`, `fmi3`, `aas`, and `digital-twin` feature flags.
- Unpacked FMU layout detection locates `modelDescription.xml` and the current host's `binaries/<platform>/` shared library.
- Dependency-free import layout validation now reports missing roots, missing `modelDescription.xml`, missing platform binary directories, missing shared-library candidates, and malformed `modelDescription.xml` root/FMI-version markers before dynamic loading is introduced.
- `.fmu` zip extraction and dynamic symbol loading are explicitly blocked pending dependency approval for archive and dynamic-loading crates.
- FMI 2.0 function-table types and safe wrapper methods exist for setup, initialization, stepping, termination, and real/integer/boolean/string scalar access.

### Task 1.2 — Implement FmuInstance
- Implement `FmuInstance` struct with safe Rust wrapper around raw FMI function pointers.
- Implement `do_step()` with error handling: translate FMI status codes to Rust `Result`.
- Implement typed variable accessors: `get_real(vr)`, `set_real(vr, value)`, and equivalents for integer, boolean, string.
- Implement `Drop` for `FmuInstance` that calls `terminate` and `free_instance`.

Current artifacts:
- `Fmi2CoSimulationInstance` wraps a raw FMI component and function table.
- `do_step()` maps non-success FMI status codes to `FmiError::FmiStatus`.
- Typed scalar getters/setters check value-reference/value length consistency.
- `Drop` calls `terminate` once and then `free_instance`.

### Task 1.3 — Co-simulation smoke test
- Create `examples/fmi-co-simulation/basic-import/` using a reference FMU from FMI Cross-Check.
- Execute 1000 consecutive `do_step` calls and verify no crash, no memory leak (valgrind/dhat), and consistent output.
- Add CI integration test using a small pre-built reference FMU committed to the test fixtures.

### Task 1.4 — Add review criteria
- Add red-team prompts for: binary incompatibility across OS, FMI version mismatch, dangling FMU state after partial initialization, concurrent FMU access.
- Add measurable acceptance criteria: 1000-step smoke test passes on Linux, macOS, and Windows.

## Phase 2 — FMI export pipeline

### Task 2.1 — modelDescription.xml generator
- Implement `ModelDescription` struct with serialization to FMI 2.0 compliant XML.
- Generate variable catalog from KairoECS component schema: scalar variables, causality (`input`/`output`/`local`), variability (`constant`/`fixed`/`tunable`/`discrete`/`continuous`).
- Generate `modelStructure` with `outputs` → `derivatives` → `initialUnknowns` dependency chain.

Current artifacts:
- `ModelDescription` and `ScalarVariable` generate initial FMI 2.0 XML with scalar variables and FMI 2.0 `<ModelStructure><Outputs><Unknown index="...">` output entries.
- `ModelDescription::validate()` rejects empty identifiers, duplicate scalar-variable names, and duplicate value references; `ModelDescription::validate_generated_fmi2_xml()` checks dependency-free generated XML markers and output structure counts; `validate_unpacked_export_layout()` checks the written unpacked FMU package shape.
- KairoECS component-schema discovery and FMI XSD validation remain open.

### Task 2.2 — FMU export build pipeline
- Implement `fmi-export` CLI or `build.rs` helper that:
  1. Generates `modelDescription.xml` from KairoECS model definition.
  2. Generates C source stubs with FMI callback functions.
  3. Compiles C stubs + Rust library into a shared library.
  4. Packages shared library + `modelDescription.xml` + resources into a zip (`.fmu`) archive.
- Validate FMU structure against FMI 2.0 schema using XML schema validation.

### Task 2.3 — OpenModelica round-trip test
- Export a simple KairoECS model (e.g., bouncing ball, damped oscillator) as an FMU.
- Load exported FMU in OpenModelica OMEdit.
- Run simulation in OpenModelica and compare trajectory to native KairoECS run.
- Verify output within 1e-6 numerical tolerance for deterministic models.

## Phase 3 — AAS and digital twin

### Task 3.1 — AAS JSON connector
- Implement `AasDescriptor` and `AasSubmodel` structs.
- Map KairoECS component topology to AAS submodel elements.
- Serialize to AAS JSON format (AAS Specification Part 1 v3.0).
- Validate output against AASX Package Explorer schema.

Current artifacts:
- `AasDescriptor`, `AasSubmodel`, and `AasProperty` serialize a minimal AAS JSON envelope behind the `aas` feature.
- `AasDescriptor::validate()` checks required descriptor/submodel/property identifiers, duplicate submodel IDs, duplicate submodel `idShort` values, and duplicate property IDs without external schema dependencies; property `semanticId` values are serialized when present.
- Schema validation against AASX Package Explorer remains open.

### Task 3.2 — Live data bridge
- Implement `DigitalTwinConnector` that subscribes to FMU output variable changes.
- Map FMU variable values to streaming topics via Track 36 contracts (Arrow RecordBatch or protobuf).
- Implement configurable sample rate and change-detection publishing (publish only on delta > epsilon).

Current artifacts:
- `DigitalTwinConnector` publishes value-reference changes when `abs(delta) > epsilon`.
- `DigitalTwinConnector::try_new()` and `try_publish_changes()` validate positive finite sample rates, finite non-negative epsilon values, topic-prefix shape, and finite publication values before returning publication records.
- Final Track 36 Arrow/protobuf integration remains open.

### Task 3.3 — State synchronization
- Implement `TwinStateSnapshot`: serialize full ECS state at a tick boundary.
- Implement `TwinStateDiff`: compute delta between two snapshots for efficient transmission.
- Implement `TwinStateApply`: apply a received state diff to synchronize the digital shadow.

Current artifacts:
- `TwinStateSnapshot`, `TwinStateDiff`, and `TwinStateSnapshot::apply()` support deterministic key ordering, checksums, changed entries, and removed entries.
- Direct ECS state serialization remains open until Track 01 serialization contracts are finalized.

## Phase 4 — FMI 3.0 support

### Task 4.1 — FMI 3.0 import runtime
- Extend `FmuInstance` to support FMI 3.0 co-simulation interface.
- Implement `fmi3InstantiateCoSimulation`, `fmi3DoStep` with variable-step support, `fmi3GetFloat64`, `fmi3SetFloat64`.
- Handle FMI 3.0 structural parameters, clocks, and event mode.

### Task 4.2 — FMI 3.0 export
- Extend `ModelDescription` generator to produce FMI 3.0 compliant XML.
- Support FMI 3.0 terminals, clocks, and scheduled execution.

## Phase 5 — Closeout

### Task 5.1 — Run quality gates
- Run `cargo test --features fmi2` on all platforms.
- Run the 1000-step co-simulation smoke test on Linux, macOS, and Windows CI.
- Validate FMU export round-trip with OpenModelica on Linux CI.
- Validate AAS JSON against schema in CI.
- Verify `cargo build --no-default-features` excludes all FMI dependencies.

### Task 5.2 — Update risk register
- Move resolved risks to mitigated.
- Promote unresolved binary compatibility or co-simulation stability risks to release blockers.
- Document FMI version support matrix and platform compatibility table.
## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Update the track registry/status surfaces: `conductor/tracks.yaml` (authoritative machine-readable registry), `conductor/tracks.md` (human index), `conductor/phase-closeout.yaml` (review ledger), `conductor/status.md` (narrative status), and `conductor/implementation-readiness.md` or `conductor/track-map.md` when readiness, ownership, dependency, gate, or wave data changes.
5. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` plus the gates listed in `test-matrix.md`.
6. Commit and push the cleaned slice, then record the commit SHA or blocker in `handoff.md`.
7. Run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` to verify recorded commits, pushed refs, and cleanup state.
8. Advance the next phase only after there is no in-scope unstaged or untracked work except documented draft satellites.