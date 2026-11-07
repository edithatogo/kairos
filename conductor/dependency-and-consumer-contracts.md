# Dependency qualification and consumer boundary

Owner: [Track 114](tracks/114-dependency-and-consumer-contracts/index.md), consuming existing dependency Tracks 70–104 and scientific integration Track 108.

## Kairos is an independent dependency

Dependency direction: CareOps Sim -> Kairos public crate/ABI/schema contracts. Kairos has no dependency on CareOps implementation, private data, repository layout or deployment. Keep healthcare pathways, staffing policies, costs, FHIR/OMOP mappings, operational calibration and local scenarios in CareOps/domain packages.

Publish a small coherent crate family and/or stable Rust facade only after package qualification. Start with the existing core/types/state/RNG/DES/ABM APIs and explicit optional telemetry features; add a facade only if measured caller friction justifies it. Minimal builds must not pull in Arrow/Polars, Python, GPU, MPI, HTTP or CareOps. Expensive integrations are named optional capabilities.

CareOps owns a thin adapter mapping domain inputs to public Kairos types and outputs to domain metrics. It pins an immutable released version or reviewed Git commit; an unqualified reference fallback is forbidden. A pin update must run installed-consumer replay, error, schema and buffer-lifetime oracles and record both source IDs. Do not copy engine code, depend on a developer's workspace path or import private Kairos internals.

Existing submodule/source layout is historical consumer machinery, not the interface contract. Replace it through a consumer-owned dependency change only after the candidate package is accepted. This Kairos packet does not silently advance CareOps/Innovate pins. Add a standalone minimal downstream Cargo fixture in Kairos, then let the real consumer exercise its own model; CI for Kairos never requires access to CareOps.

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
| Inference | Tract/ort are candidates, not accepted native inference merely because a feature is named. | Check supported operators, licence, model fixtures, build artefacts, acceleration and measured error/latency. Rust adapter owns execution/error contracts. No Python core fallback. |
| GPU | Existing wgpu/cudarc. | Consolidate GPU ownership/transfer abstractions; do not add another graphics stack without a model requiring it. |
| FMI | Existing FMI loader and Track91 evaluation; consider maintained Rust FMI/fmusim tooling and Reference-FMUs. | Independent FMUs are oracles. Adopt a library only after checking supported FMI features, licence, lifecycle/error behaviour and maintenance; avoid duplicate native loading code if replacement passes. |
| Transport/tracing | Tokio/tonic/prost already support accepted transport; tracing and WebSocket qualification tracks exist. | Enable only for concrete consumers; no automatic dependency adoption from a completed evaluation packet. |
| Bindings | Existing C ABI, PyO3/maturin and wasm-bindgen surfaces. | Generate marshalling where practical; one authoritative Rust implementation. Package test each native artefact. Keep C ABI for languages where it reduces hand-maintained bridge logic. |

## Adoption evidence and maintenance

For any change, record upstream activity/security/licence/API stability, exact selected version/revision, consumer need, eliminated custom code, compile time and binary size, runtime/semantic regression, target support and exit strategy. Prefer one maintained implementation over several overlapping wrappers. A qualification decision may be defer/no-go; that is not runtime adoption.

Automation should open reviewable dependency changes, run locked feature/platform consumers and produce an evidence diff. Security fixes get priority. Behaviour-changing RNG, numerics, ABI/schema or checkpoint upgrades require explicit compatibility review; do not auto-merge them. Retain reproducible locks, SBOM/provenance and existing release gates.

