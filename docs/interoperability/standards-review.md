# Interoperability Standards Review

This page records which interoperability standards KairoECS is aligning with,
which ones are only partial or deferred, and which ones are intentionally not
claimed. It is a review artifact, not a compatibility promise.

Concrete gate artifacts:

- `docs/interoperability/standards-mapping.md` satisfies the `standards-mapping`
  gate with claim-by-claim surfaces, evidence, and release guards.
- `docs/interoperability/adr-recommendations.md` satisfies the
  `adr-recommendations` gate with ADR thresholds and recommendation IDs.

## Status vocabulary

| Label | Meaning for release language |
|---|---|
| Supported | KairoECS has a checked-in implementation or contract surface that can be referenced, with evidence named below. |
| Partial | KairoECS aligns with a subset of the standard or convention, but missing behavior must be named in the same row. |
| Deferred | The standard is a planned or plausible bridge target, but current release language must not claim support. |
| Unsupported | The standard is explicitly outside the current compatibility claim; use only as a comparison or teaching reference. |

## Standards mapping

| Standard | Label | KairoECS surface | Evidence in repo | Missing behavior or release guard |
|---|---|---|---|---|
| DEVS | Partial | Event ordering, scheduler vocabulary, and deterministic replay concepts. | `conductor/tracks/26-interoperability-standards-review/spec.md`; `docs/trustworthy-simulation/replay-and-seeds.md`. | No DEVS model import/export, coupled-model protocol, DEVS simulator conformance suite, or runtime compatibility claim. |
| FMI/FMU | Partial | FMI 2.0 unpacked FMU layout checks, `modelDescription.xml` presence checks, function-table types, lifecycle wrapper, and unpacked export layout generation. | `docs/fmi-digital-twin/import-guide.md`; `docs/fmi-digital-twin/export-guide.md`; `crates/kairo-ecs-fmi/src/import/fmu_loader.rs`; `crates/kairo-ecs-fmi/src/export/packager.rs`. | Dynamic symbol loading, archive extraction, deterministic `.fmu` packaging, FMI XSD validation, compiled platform binaries, and OpenModelica round-trip comparison remain beta/1.0 work. |
| SBML | Deferred | Future continuous or biochemical model bridge target. | `conductor/tracks/26-interoperability-standards-review/spec.md`; this standards review. | No SBML parser, writer, semantic mapping, unit handling, solver bridge, or fixture coverage. |
| CellML | Deferred | Future continuous model bridge target. | `conductor/tracks/26-interoperability-standards-review/spec.md`; this standards review. | No CellML parser, writer, variable/unit mapping, solver bridge, or fixture coverage. |
| OpenTelemetry semantic conventions | Partial | Trace, span, log, and metric naming guidance only. | `docs/debugging/trace-format.md`; `docs/streaming/stream-schema.md`; this standards review. | No native OpenTelemetry exporter, OTLP payload contract, resource/span semantic-convention matrix, or semantic-convention drift monitor. |
| Arrow C Data Interface | Partial | Field-level Arrow type contract for `kairo_ecs.event_log.v1` event-log records. | `crates/kairo-ecs-arrow/src/lib.rs`; `crates/kairo-ecs-arrow/tests/schema_compatibility.rs`; `schemas/arrow/event_log_v1.schema.json`. | No exported ArrowArray/ArrowSchema FFI boundary or zero-copy cross-language C Data Interface fixture yet. |
| Arrow IPC | Deferred | Integration target for event logs, stream payloads, and time-travel traces. | `docs/streaming/stream-schema.md`; `docs/debugging/trace-format.md`; `conductor/tracks/04-analyst-kairo-ecs-arrow/handoff.md`. | Current Track 04 roundtrip uses dependency-light smoke bytes, not full Arrow IPC. Release language must not claim Arrow IPC support until IPC serialization and reader fixtures exist. |
| Parquet | Deferred | Planned persisted analytical output for runs, comparisons, VVUQ, and replay artifacts. | `docs/trustworthy-simulation/replay-and-seeds.md`; `conductor/trustworthy-simulation.md`; `conductor/verification-validation-uncertainty.md`. | No Parquet writer, schema evolution tests, compression policy, or reader compatibility fixtures. |

## Ecosystem references

| Ecosystem | Label | Use allowed in docs | Release guard |
|---|---|---|---|
| Mesa, Agents.jl, MASON, NetLogo | Unsupported | ABM teaching, migration examples, and vocabulary comparison. | Do not claim behavioral equivalence or model import/export. |
| SimPy, simmer, ConcurrentSim.jl, SimSharp | Unsupported | DES teaching, migration examples, and vocabulary comparison. | Do not claim API compatibility or trajectory equivalence. |
| AnyLogic-style multimethod modeling | Unsupported | User mental model for hybrid systems. | Do not claim project or model interchange. |

## Release-impacting assertions

The following assertions matter for release review:

- Any Arrow claim must name the exact surface: Arrow C Data Interface,
  Arrow IPC, or Parquet. Field-level Arrow type alignment is not the same as
  Arrow IPC or Parquet support.
- Arrow schema changes to `kairo_ecs.event_log.v1` can affect downstream
  bindings, benchmark outputs, stream adapters, replay artifacts, and release
  compatibility notes.
- OpenTelemetry semantic-convention alignment must not be written as native
  OTel or OTLP export support unless an exporter and fixture suite exist.
- FMI/FMU support must name the exact maturity level. Current evidence supports
  partial unpacked-layout and lifecycle/export scaffolding only; arbitrary
  third-party FMU execution is not supported.
- SBML, CellML, Arrow IPC, and Parquet remain deferred until concrete
  interchange contracts and conformance fixtures exist.
- unsupported ecosystem references are comparison aids only and cannot appear in
  compatibility promises.

## Red-team checks

| Prompt | Required answer before release language is allowed |
|---|---|
| Does the claim imply runtime compatibility with another simulator? | Name the standard, the exact KairoECS surface, and the passing conformance fixture. |
| Does the claim say Arrow without a surface? | Rewrite to Arrow C Data Interface field alignment, Arrow IPC, or Parquet. |
| Does the claim mention OpenTelemetry? | State whether this is naming guidance or an implemented exporter. |
| Does the claim mention FMI/FMU? | State whether this is unpacked-layout validation, export layout generation, lifecycle wrapper support, or full FMU execution. |
| Does the claim mention SBML or CellML? | Mark it deferred unless a parser/writer and solver bridge exist. |

## Local validation

Run the Track 26 validator after editing this page:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/26-interoperability-standards-review/validate-standards-review.ps1
```
