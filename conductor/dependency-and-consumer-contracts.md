# Dependency qualification and consumer boundary

Owner: [Track 114](tracks/114-dependency-and-consumer-contracts/index.md), consuming existing dependency Tracks 70–104 and scientific integration Track 108.

## Kairos is an independent dependency

The [ecosystem decision](../docs/adr/0011-ecosystem-boundaries.md) and
[actual consumer inventory](ecosystem-consumers.md) refine this scope. UOGTO
defines game semantics; [Paignia](https://github.com/edithatogo/paignia) receives
the existing game seed as `paignia-core`/`paignia-solve`, and its separate optional
adapter uses Kairos. The old incubator becomes a consumer. Do not add game
solvers or HEOR valuation to the minimal engine. Generic movement/allocation/use
contracts belong here; domain movement policy and economic valuation belong
above them. [Bounded implementation tasks](execution/README.md) cover this
work without another coordinator or approval process.

Dependency direction: CareOps Sim -> Kairos public crate/ABI/schema contracts. Kairos has no dependency on CareOps implementation, private data, repository layout or deployment. Keep healthcare pathways, staffing policies, costs, FHIR/OMOP mappings, operational calibration and local scenarios in CareOps/domain packages.

Publish a small coherent crate family and/or stable Rust facade only after package qualification. Start with the existing core/types/state/RNG/DES/ABM APIs and explicit optional telemetry features; add a facade only if measured caller friction justifies it. Minimal builds must not pull in Arrow/Polars, Python, GPU, MPI, HTTP or CareOps. Expensive integrations are named optional capabilities.

CareOps owns a thin adapter mapping domain inputs to public Kairos types and outputs to domain metrics. It pins an immutable released version or reviewed Git commit; an unqualified reference fallback is forbidden. A pin update must run installed-consumer replay, error, schema and buffer-lifetime oracles and record both source IDs. Do not copy engine code, depend on a developer's workspace path or import private Kairos internals.

Existing submodule/source layout is historical consumer machinery, not the interface contract. Replace it through a consumer-owned dependency change only after the candidate package is accepted. This Kairos packet does not silently advance CareOps/Innovate pins. Add a standalone minimal downstream Cargo fixture in Kairos, then let the real consumer exercise its own model; CI for Kairos never requires access to CareOps.

CareOps's existing `careops-core` is the shared healthcare layer; do not create a
second healthcare library merely to remove its ED-specific helpers. The
[8 November source review](design/careops-composition-plan-20261020.md#paignia-and-careops-boundary-review)
maps concrete scheduler/RNG/hazard/allocation duplication and generic experiment
lifecycle reuse to existing owners K-C02/K-C04/K-C05. First qualify the narrow
experiment API with actual ED/logistics cancellation and recovery fixtures;
reuse a library layer, not CLI internals or an entire calibration stack.

## Reuse decisions

| Area | Current position / candidate | Qualification decision |
|---|---|---|
| Parallel execution | Rayon is already adopted for independent sweeps (78). | Reuse for persistent PDES workers only after ordering/lookahead tests and measured crossover; custom fixed workers remain an alternative if simpler. |
| Benchmarks/properties | Criterion (101) and proptest (96) already exist. | Consolidate duplicate timing/scenario tooling; add semantic mutation tests and model checking where concurrent state exists. Never equate more test counts with stronger validation. |
| RNG/distributions | rand/ChaCha and rand_distr already have qualification tracks. | Preserve explicit algorithms, streams and output versions; adopt fixes through replay contracts. |
| Data | Arrow/Polars, schema and C Data adapters already exist. | Keep optional; use maintained formats and ownership APIs. Respect the existing pinned upstream patch/publication constraint until qualified resolution. |
| Graph/spatial | petgraph, geo and rstar are already present. | Reuse routing/spatial primitives while retaining public deterministic tie-breaking and access semantics. |
| ECS/storage | Existing custom storage; evaluate hecs/bevy_ecs only for a demonstrated maintenance or performance gain. | Test generational/full-width IDs, deterministic iteration, snapshots, borrow boundaries and adapter cost. A crate-local forbid(unsafe_code) does not automatically ban unsafe in dependencies; audit safety boundaries explicitly. Avoid a disruptive framework rewrite without evidence. |
| ODE/numerical | Track102 deferred ode_solvers; Track110 owns the existing RK4 correctness defect. | Fix the numerical contract first. Qualify a maintained Rust solver when the derivative/event API and tolerances are specified; a dependency upgrade cannot repair an incorrectly specified method by itself. |
| Optimization/game theory | Clarabel already integrated (85); argmin qualification is separate. | Use the supported solver for actual incentive/optimization constraints, with independent residual checks. Avoid bespoke general solvers where a compatible maintained Rust implementation is adequate. |
| Inference | Track 89 rejected Tract for its evaluated scope. The opt-in Rust ONNX CPU profile has separate fixed-shape float32 qualification. | Reuse the named CPU profile within its tested model limits. [ONNX contract and evidence](../docs/adr/0026-onnx-cpu-inference.md) |
| GPU | Existing wgpu/cudarc. | Consolidate GPU ownership/transfer abstractions; do not add another graphics stack without a model requiring it. |
| FMI | Existing FMI loader and Track91 evaluation; consider maintained Rust FMI/fmusim tooling and Reference-FMUs. | Independent FMUs are oracles. Adopt a library only after checking supported FMI features, licence, lifecycle/error behaviour and maintenance; avoid duplicate native loading code if replacement passes. |
| Transport/tracing | Tokio/tonic/prost already support accepted transport; tracing and WebSocket qualification tracks exist. | Enable only for concrete consumers; no automatic dependency adoption from a completed evaluation packet. |
| Bindings | Existing C ABI, PyO3/maturin and wasm-bindgen surfaces. | Generate marshalling where practical; one authoritative Rust implementation. Package test each native artefact. Keep C ABI for languages where it reduces hand-maintained bridge logic. |

## Tracks 70–104 disposition — 25 November 2026

The [track records](tracks.md) and the decisions below were reconciled against
`6316b4d7a2158865e0f3533ff3abe1cd4856099b`. Reopen an evaluation when an application demonstrates the
required workload and supplies its acceptance oracle.

| Tracks | Disposition and current boundary |
| --- | --- |
| 70–83, 85–87, 93–96 | Retain the adopted Polars, RNG, routing, serialization, CLI, Rayon, FMU archive/metadata, transport, solver, spatial, integrity and property-testing components in their qualified features. The owning track receipts retain versions and compatibility limits. |
| 84, 88, 89, 91, 92, 99 | Keep the recorded rejections of the evaluated hecs, rsmpi replacement, Tract, rust-fmi, Sobol–Burley and proptest-state-machine migrations. The independently qualified MPI feature, ONNX CPU adapter and built-in Sobol sequence follow their own contracts. A crate's `forbid(unsafe_code)` applies to that crate. An ECS replacement would still need to preserve stable identity, iteration, snapshots and borrowing. [Native transport](tracks/49-distributed-mpi-grpc-state-synchronization/native-runtime-qualification-20261103.md) · [Scientific scope](scientific-completion.md) |
| 90, 97, 98, 100, 102–104 | Defer Argmin, additional tracing and model checking, a generic ODE integrator and the WebSocket stack until their owning workloads and entry conditions are met. [Entry conditions](dependency-sequencing.md) |
| 101 | Retain Criterion for microbenchmarks and the existing specialist scaling harnesses. [Consolidation result](tracks/101-criterion-consolidation/handoff.md) |

The CareOps movement case uses the adopted routing and occupancy APIs. The
paired PDES ring supplies the existing cost decision for persistent workers.
The FMU importer reuses ZIP/XML handling with the accepted archive and
metadata validation. Changing any of these choices requires the same locked
consumer behavior, build and licence/security review from its owning track.
[Movement evidence](tracks/114-dependency-and-consumer-contracts/handoff.md#installed-careops-movement--23-november-2026) ·
[Worker measurements](archive/111-measured-performance/handoff.md#matched-ring-and-worker-reuse) ·
[Archive decision](tracks/79-zip-robustness/handoff.md) ·
[Metadata decision](tracks/80-quick-xml-robustness/handoff.md)

## Adoption evidence and maintenance

For any change, record upstream activity/security/licence/API stability, exact selected version/revision, consumer need, eliminated custom code, compile time and binary size, runtime/semantic regression, target support and exit strategy. Prefer one maintained implementation over several overlapping wrappers. A qualification decision may be defer/no-go; that is not runtime adoption.

Automation should open reviewable dependency changes, run locked feature/platform consumers and produce an evidence diff. Security fixes get priority. Behaviour-changing RNG, numerics, ABI/schema or checkpoint upgrades require explicit compatibility review; do not auto-merge them. Retain reproducible locks, SBOM/provenance and existing release gates.

