# Track 26 ADR Recommendations

This artifact is the concrete evidence for the `adr-recommendations` gate. It
does not create binding ADRs by itself; it states when a standards decision is
small enough for Track 26 guidance and when a formal ADR is required before
public compatibility language changes.

## ADR threshold

Open or update an ADR before release language changes when any of these are
true:

- a Partial, Deferred, or Unsupported item would be promoted to Supported;
- a standard requires a new public schema, wire format, FFI boundary, or
  package-level compatibility promise;
- a standard version pin, minimum version, or semantic-convention version would
  be part of the public contract;
- an ecosystem comparison would become a runtime, import/export, or behavioral
  compatibility claim.

## Recommendations

| Recommendation ID | Standard or surface | Current label | Recommendation | ADR required before claim changes? | Trigger |
|---|---|---|---|---|---|
| ADR-026-001 | DEVS | Partial | Keep DEVS as conceptual scheduler/replay alignment for alpha. | Yes | Claiming DEVS model import/export, coupled-model execution, or simulator conformance. |
| ADR-026-002 | FMI/FMU | Partial | Keep current claim limited to unpacked-layout validation, lifecycle wrapper scaffolding, and unpacked export layout generation. | Yes | Claiming arbitrary third-party FMU execution, deterministic `.fmu` packaging, FMI XSD conformance, or OpenModelica round-trip compatibility. |
| ADR-026-003 | SBML | Deferred | Keep SBML deferred until a dedicated hybrid/continuous bridge design exists. | Yes | Adding parser/writer surfaces, units mapping, solver bridge, or fixture-backed support language. |
| ADR-026-004 | CellML | Deferred | Keep CellML deferred until a dedicated hybrid/continuous bridge design exists. | Yes | Adding parser/writer surfaces, variable/unit mapping, solver bridge, or fixture-backed support language. |
| ADR-026-005 | OpenTelemetry semantic conventions | Partial | Use OpenTelemetry only as naming guidance for trace/log/metric concepts. | Yes | Claiming native OTel export, OTLP compatibility, resource/span semantic-convention conformance, or collector integration. |
| ADR-026-006 | Arrow C Data Interface | Partial | Keep the current claim at field-level Arrow type alignment for `kairo_ecs.event_log.v1`. | Yes | Adding ArrowArray/ArrowSchema FFI boundaries or zero-copy cross-language fixtures. |
| ADR-026-007 | Arrow IPC | Deferred | Keep Arrow IPC deferred until real serialization and reader fixtures replace smoke-byte evidence. | Yes | Claiming Arrow IPC read/write support, stream compatibility, or cross-runtime IPC interchange. |
| ADR-026-008 | Parquet | Deferred | Keep Parquet deferred until analytical persistence has writer, reader, compression, and schema-evolution fixtures. | Yes | Claiming Parquet output, compatibility with analytics engines, or long-term persisted artifact guarantees. |
| ADR-026-009 | ABM/DES ecosystem comparisons | Unsupported | Keep Mesa, Agents.jl, MASON, NetLogo, SimPy, simmer, ConcurrentSim.jl, SimSharp, and AnyLogic-style references as teaching and migration comparisons only. | Yes | Claiming runtime compatibility, model conversion, behavioral equivalence, or import/export with those ecosystems. |

## Initial ADR backlog

| Priority | ADR candidate | Owning follow-up | Blocked until |
|---|---|---|---|
| High | Arrow event-log interchange contract | Track 04 plus API governance | Arrow IPC or Arrow C Data Interface fixtures exist. |
| High | FMI/FMU execution maturity and compatibility boundary | Track 38 plus API governance | Dynamic loading, platform binaries, XSD validation, and OpenModelica round-trip evidence exist. |
| Medium | OpenTelemetry exporter and semantic-convention version policy | Streaming/observability follow-up | Native exporter and OTLP fixture plan exist. |
| Medium | Parquet analytical artifact schema | VVUQ/replay plus Arrow follow-up | Writer/reader fixtures and schema-evolution policy exist. |
| Low | SBML/CellML hybrid bridge scope | Future hybrid/continuous modeling track | Parser/writer and solver-bridge requirements are scoped. |

## Non-ADR decisions

These do not need a formal ADR as long as the current labels remain unchanged:

- documenting DEVS terminology as conceptual alignment;
- documenting OpenTelemetry semantic conventions as naming guidance;
- documenting SBML, CellML, Arrow IPC, and Parquet as deferred;
- using ABM/DES ecosystem names only for teaching and migration comparison.

