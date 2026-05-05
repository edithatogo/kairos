# Interoperability Standards Review

This page records which interoperability standards KairoECS is aligning with,
which ones are only conceptual references, and which ones are intentionally
deferred. It is a review artifact, not a compatibility promise.

## Standards in scope

| Standard or ecosystem | KairoECS mapping | Status |
|---|---|---|
| DEVS | DES event ordering, scheduler vocabulary, and replay terminology | Supported as conceptual alignment |
| FMI / FMU | Digital-twin and co-simulation interchange | Deferred |
| SBML | Continuous or biochemical model exchange | Deferred |
| CellML | Continuous model exchange | Deferred |
| OpenTelemetry semantic conventions | Trace and log naming guidance | Partial |
| Arrow C Data Interface | Cross-language data exchange for telemetry and event buffers | Supported |
| Arrow IPC | Portable telemetry and event-log exchange | Supported |
| Parquet | Persisted analytical output for runs and comparisons | Supported |
| Mesa | ABM teaching and migration reference point | Conceptual |
| Agents.jl | ABM teaching and migration reference point | Conceptual |
| MASON | ABM teaching and migration reference point | Conceptual |
| NetLogo | ABM teaching and migration reference point | Conceptual |
| SimPy | DES teaching and migration reference point | Conceptual |
| simmer | DES teaching and migration reference point | Conceptual |
| ConcurrentSim.jl | DES teaching and migration reference point | Conceptual |
| SimSharp | DES teaching and migration reference point | Conceptual |
| AnyLogic-style multimethod modeling | User mental model for hybrid systems | Conceptual |

## Mapping notes

### DEVS

KairoECS uses DEVS as a vocabulary match for event ordering, atomic versus
coupled model thinking, and deterministic replay. The repo does not claim DEVS
import/export or simulator-to-simulator runtime compatibility.

### FMI / FMU

FMI and FMU are future bridge targets for digital-twin and co-simulation work.
At this stage, they are not supported artifacts in the repo.

### SBML and CellML

SBML and CellML are useful references for continuous and hybrid models, but the
current codebase is still DES/ABM-first. These standards are documented so
future bridge work has a named target, not because the current implementation
can load or emit them.

### OpenTelemetry

OpenTelemetry semantic conventions are useful for naming traces, spans, and
logs. The repo may align terminology with OTel where it helps observability, but
that does not mean KairoECS emits native OTel payloads.

### Arrow

Arrow C Data Interface, Arrow IPC, and Parquet are the first-class exchange
targets for telemetry and result data. These are the standards that most
directly affect cross-language parity and benchmark reproducibility.

### DES and ABM ecosystems

Mesa, Agents.jl, MASON, NetLogo, SimPy, simmer, ConcurrentSim.jl, and SimSharp
are listed as teaching and migration references. They help with terminology,
example framing, and docs, but they are not compatibility claims.

## Gaps

- No runtime DEVS import/export.
- No FMI/FMU co-simulation bridge.
- No SBML or CellML loader/emitter.
- No native OpenTelemetry exporter contract.
- No claim of semantic equivalence with Mesa, Agents.jl, MASON, NetLogo,
  SimPy, simmer, ConcurrentSim.jl, or SimSharp.
- No claim that conceptual mappings imply behavioral equivalence.

## Release-impacting assertions

The following assertions matter for release review:

- Any claim that KairoECS supports Arrow exchange must name the exact surface:
  C Data Interface, Arrow IPC, or Parquet.
- Any claim that KairoECS is interoperable with another simulator must say
  whether it is a conceptual mapping, a data-exchange mapping, or a runtime
  compatibility claim.
- Any Arrow schema change can affect downstream bindings and benchmark outputs
  and therefore needs review against the track's compatibility and reproducibility
  expectations.
- Any OpenTelemetry alignment must not be written as a native OTel export claim
  unless an exporter exists.
- Any future FMI/FMU, SBML, or CellML bridge must be treated as deferred work
  until a concrete interchange contract exists.

## How to read this page

- Supported means the repo already treats the standard as a real exchange
  surface or primary naming target.
- Partial means only the vocabulary or a subset of the concepts are aligned.
- Conceptual means the standard is helpful for teaching or documentation, but
  not for runtime compatibility claims.
- Deferred means the mapping is important enough to name now, but not yet
  implemented.
