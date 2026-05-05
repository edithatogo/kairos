# Agent Contract: fmi-agent

## Track

Track 38: FMI/FMU & Digital Twin Bridge

## Owned paths

- `crates/kairo-ecs-fmi/`
- `docs/fmi-digital-twin/`
- `examples/fmi-co-simulation/`
- `conductor/tracks/38-fmi-fmu-digital-twin-bridge/`

## Required handoff

- Summary of artifacts produced (import runtime, export pipeline, AAS connector, digital twin bridge).
- FMI version support matrix (2.0 CS, 3.0 CS, model exchange deferral).
- Platform compatibility table: which OS/arch combinations support FMU import and export.
- Reference FMU test results: which FMI Cross-Check FMUs pass the 1000-step smoke test.
- OpenModelica round-trip benchmark results and numerical tolerance analysis.
- AAS descriptor schema validation results.
- Risks discovered and unresolved questions.

## Prohibited changes without ADR

- Public Rust APIs in `kairo-ecs-core` or `kairo-ecs-runtime`.
- C ABI signatures owned by Track 02.
- Streaming pipeline contracts owned by Track 36.
- ECS component schema serialization format.
- Package names or registry publication policy.

## Feature flag governance

- `fmi2`: gates FMI 2.0 import/export; requires `libloading` and XML serialization.
- `fmi3`: gates FMI 3.0 import/export; extends `fmi2` support.
- `aas`: gates AAS JSON connector; requires `serde_json`.
- `digital-twin`: gates live data bridge and state synchronization; depends on `fmi2` or `fmi3` + streaming contracts from Track 36.

## FMI standard compliance

- FMI 2.0: target FMI 2.0.4 co-simulation interface. Model exchange deferred to post-v1.0.
- FMI 3.0: target FMI 3.0 co-simulation interface. Scheduled execution and clocks deferred to post-v1.0.
- FMU binary interface: strictly follow the C ABI specified in FMI 2.0.4 Section 2.1 and FMI 3.0 Section 4.1.
- `modelDescription.xml`: must validate against `fmi2ModelDescription.xsd` (FMI 2.0) and `fmi3ModelDescription.xsd` (FMI 3.0).

## Integration points

- Consumes C ABI primitives from Track 02 for FMU function pointer types and calling conventions.
- Consumes streaming topic publication from Track 36 for live data bridge.
- Consumes ECS component value serialization from Track 01 for FMU export variable mapping.
- Provides FMU import as an ECS system plugin (`FmuCoSimulationSystem`) that steps FMU slaves at tick boundaries.
