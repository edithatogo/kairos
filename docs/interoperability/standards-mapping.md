# Track 26 Standards Mapping

This artifact is the concrete evidence for the `standards-mapping` gate. It
keeps release language tied to exact KairoECS surfaces and names the missing
behavior that prevents stronger compatibility claims.

## Gate rule

Every interoperability claim must name:

- the exact standard or ecosystem reference;
- the current Track 26 label: Supported, Partial, Deferred, or Unsupported;
- the KairoECS surface that exists today;
- the evidence file that backs the claim;
- the missing behavior or release guard that limits the claim.

## Primary standards

| Standard | Current label | Claim allowed now | Current KairoECS surface | Evidence | Missing behavior or release guard |
|---|---|---|---|---|---|
| DEVS | Partial | Conceptual alignment with event ordering, scheduler terminology, and deterministic replay. | Scheduler and replay vocabulary. | `docs/trustworthy-simulation/replay-and-seeds.md`; `conductor/tracks/26-interoperability-standards-review/spec.md`. | No DEVS model import/export, coupled-model protocol, simulator conformance suite, or runtime compatibility claim. |
| FMI/FMU | Partial | Unpacked FMU layout validation, lifecycle wrapper scaffolding, and unpacked export layout generation. | `kairo-ecs-fmi` import/export scaffolds and docs. | `docs/fmi-digital-twin/import-guide.md`; `docs/fmi-digital-twin/export-guide.md`; `crates/kairo-ecs-fmi/src/import/fmu_loader.rs`; `crates/kairo-ecs-fmi/src/export/packager.rs`. | No dynamic symbol loading, `.fmu` archive packaging, FMI XSD validation, compiled platform binaries, or OpenModelica round-trip comparison. |
| SBML | Deferred | Future bridge target only. | None beyond this review artifact. | `conductor/tracks/26-interoperability-standards-review/spec.md`; `docs/interoperability/standards-review.md`. | No parser, writer, semantic mapping, unit handling, solver bridge, or fixture coverage. |
| CellML | Deferred | Future bridge target only. | None beyond this review artifact. | `conductor/tracks/26-interoperability-standards-review/spec.md`; `docs/interoperability/standards-review.md`. | No parser, writer, variable/unit mapping, solver bridge, or fixture coverage. |
| OpenTelemetry semantic conventions | Partial | Naming guidance for trace, span, log, and metric concepts only. | Debug trace and streaming schema docs. | `docs/debugging/trace-format.md`; `docs/streaming/stream-schema.md`; `docs/interoperability/standards-review.md`. | No native OpenTelemetry exporter, OTLP payload contract, resource/span semantic-convention matrix, or semantic-convention drift monitor. |
| Arrow C Data Interface | Partial | Field-level Arrow type alignment for `kairo_ecs.event_log.v1`. | Arrow event-log schema and compatibility tests. | `crates/kairo-ecs-arrow/src/lib.rs`; `crates/kairo-ecs-arrow/tests/schema_compatibility.rs`; `schemas/arrow/event_log_v1.schema.json`. | No exported ArrowArray/ArrowSchema FFI boundary or zero-copy cross-language C Data Interface fixture. |
| Arrow IPC | Deferred | Future interchange target for event logs, stream payloads, and time-travel traces. | Dependency-light smoke-byte roundtrip only. | `docs/streaming/stream-schema.md`; `docs/debugging/trace-format.md`; `conductor/tracks/04-analyst-kairo-ecs-arrow/handoff.md`. | No Arrow IPC serialization, reader fixture, schema-evolution fixture, or cross-runtime IPC compatibility test. |
| Parquet | Deferred | Future persisted analytical output target. | None beyond replay and VVUQ planning docs. | `docs/trustworthy-simulation/replay-and-seeds.md`; `conductor/trustworthy-simulation.md`; `conductor/verification-validation-uncertainty.md`. | No Parquet writer, reader fixture, compression policy, schema evolution test, or downstream analytics compatibility test. |

## Ecosystem references

These rows are the unsupported ecosystem boundary for Track 26.

| Ecosystem reference | Current label | Claim allowed now | Release guard |
|---|---|---|---|
| Mesa, Agents.jl, MASON, NetLogo | Unsupported | ABM teaching, migration examples, and vocabulary comparison. | Do not claim model import/export, behavioral equivalence, or API compatibility. |
| SimPy, simmer, ConcurrentSim.jl, SimSharp | Unsupported | DES teaching, migration examples, and vocabulary comparison. | Do not claim API compatibility, trajectory equivalence, or process-model import/export. |
| AnyLogic-style multimethod modeling | Unsupported | Hybrid-simulation mental model only. | Do not claim project interchange, runtime compatibility, or model conversion. |

## Release language matrix

| Phrase in outward-facing docs | Required rewrite |
|---|---|
| "Arrow support" | Name one exact surface: Arrow C Data Interface field alignment, Arrow IPC, or Parquet. |
| "OpenTelemetry support" | Say "OpenTelemetry semantic-convention naming guidance" unless a native exporter and OTLP fixtures exist. |
| "FMI/FMU support" | Name the exact maturity: unpacked-layout validation, lifecycle wrapper scaffold, export layout generation, or full FMU execution. |
| "DEVS compatible" | Say "DEVS-informed scheduler/replay terminology" until import/export and conformance fixtures exist. |
| "SBML" or "CellML" | Mark as deferred unless parser, writer, units, solver bridge, and fixtures exist. |
| "compatible with Mesa/SimPy/AnyLogic" | Rewrite as teaching or migration comparison only. |
