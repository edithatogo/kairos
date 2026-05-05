# Track 38: FMI/FMU & Digital Twin Bridge

## Purpose

Industry-standard co-simulation interoperability. Import FMUs (Functional Mock-up Units) as co-simulation slaves so KairoECS can orchestrate multi-physics simulations. Export KairoECS as an FMU so external tools (Simulink, Dymola, OpenModelica) can consume it. Build digital twin infrastructure: asset administration shell (AAS), live data connector, state synchronization, shadow/deployment model.

## Why this track exists

Industrial simulation ecosystems rely on the FMI standard (FMI 2.0 and 3.0) for tool-agnostic model exchange and co-simulation. Without FMI import/export, KairoECS cannot interoperate with MATLAB/Simulink, Dymola, OpenModelica, or any of the 170+ FMI-compatible tools. Digital twin adoption further requires structured metadata (AAS), live data connectivity, and state synchronization between physical assets and virtual models. This track makes KairoECS a first-class citizen in the industrial co-simulation landscape.

## Primary subagent

`fmi-agent`

## Dependencies

- Track 26: Interoperability Standards Review — provides FMI 2.0/3.0 specification analysis, AAS metamodel review, and co-simulation protocol assessment.
- Track 36: Streaming Data Pipeline — provides live telemetry bridge for digital twin data connector.
- Track 02: FFI Bridge — provides C ABI primitives for FMU interface (FMUs are C shared libraries).
- Track 01: Heart/KairoECS Core — consumes ECS state for FMU export and drives FMU import as simulation slave.

## Owned paths

```text
crates/kairo-ecs-fmi/, docs/fmi-digital-twin/, examples/fmi-co-simulation/
```

## Blocked paths

```text
crates/kairo-ecs-core/ — owned by Track 01 (core state management)
crates/kairo-ecs-ffi/ — owned by Track 02 (C ABI surface)
crates/kairo-ecs-streaming/ — owned by Track 36 (data pipeline)
```

## Inputs

- FMI 2.0.4 and FMI 3.0 specification documents.
- Standards review of FMI, SSP, and AAS from Track 26.
- Streaming data bridge contracts from Track 36.
- C ABI primitives from Track 02 (`extern "C"` function pointer patterns, safe wrapping).
- Reference FMUs from FMI Cross-Check repository for compliance testing.

## Outputs

- `crates/kairo-ecs-fmi/`: import runtime (load FMU shared library, instantiate co-simulation slave, step through `fmi2DoStep`/`fmi3DoStep`) and export build pipeline (wrap KairoECS as an FMU zip archive).
- FMI import: `FmuInstance` struct with `do_step()`, `get_real()`, `set_real()`, `get_integer()`, `set_integer()`, `get_boolean()`, `set_boolean()`, `get_string()`, `set_string()` methods.
- FMI export: `fmi-export` CLI subcommand or build.rs that generates `modelDescription.xml`, source stubs, and packages a compliant FMU.
- AAS JSON connector: `AasDescriptor` struct serializing to AAS JSON schema; `AasSubmodel` for KairoECS component topology.
- Live data bridge: `DigitalTwinConnector` that maps FMU output variables to streaming topics via Track 36 contracts.
- `docs/fmi-digital-twin/`: FMI import/export guide, AAS mapping reference, digital twin deployment model.
- `examples/fmi-co-simulation/`: example co-simulating a KairoECS model with a third-party FMU.

## Acceptance criteria

- FMU import loads a co-simulation FMU (FMI 2.0 CS), instantiates it, and executes 1000 consecutive `doStep` calls without crash, memory leak, or state corruption.
- FMU export: KairoECS model is packaged as an FMU; the FMU loads in OpenModelica OMEdit and produces trajectory output matching the native KairoECS run within numerical tolerance (1e-6 for deterministic models).
- AAS descriptor JSON validates against the AASX Package Explorer schema (AAS Specification Part 1 v3.0).
- Live data bridge publishes FMU output variable changes to the streaming pipeline at the configured sample rate.
- All FMI functionality is gated behind a `fmi` Cargo feature flag.

## Non-goals

- Implementing FMI model exchange (ME) semantics in the initial release (co-simulation CS only).
- Full SSP (System Structure and Parameterization) support.
- GUI-based FMU import wizard or visual co-simulation editor.
- Certifying KairoECS as an FMI Reference FMU.
- Embedding an OPC UA server (delegated to external adapter).

## Release implications

- Release-critical for industrial and digital-twin users. Blocks v1.0 if industrial adoption is a stated goal.
- Gated behind `fmi` feature flag; headless/scientific users are not affected.
- FMU export build pipeline must be reproducible: identical KairoECS model → identical FMU binary.
- AAS descriptor is published alongside release artifacts for digital twin consumers.

## Status

Planned
