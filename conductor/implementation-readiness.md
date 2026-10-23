# KairoECS Implementation Readiness

> **Integrated planning registry — 21 October 2026:** 70 tracks: 60 recorded Done,
> 7 In Progress (48–53, 55) and 3 Planned (67–69). The 20 October audit's 67-track
> count is a historical snapshot before those registrations. Tracks62–66 have
> disputed full-completion claims; recorded status is not exercised capability.
> The [audit](capability-reconciliation-20261020.md) and
> [composition plan](design/careops-composition-plan-20261020.md) are incorporated
> together. Implementation and hardware gates remain pending; no Done status is
> advanced by this planning integration.

Last verified: 2026-10-18

## Purpose

The Conductor setup is complete, but implementation should move through explicit readiness levels. This avoids confusing "a workflow skipped because the package does not exist yet" with "the track is healthy", and it keeps GitHub and registry work tied to track maturity instead of guesswork.

## Readiness levels

| Level | Meaning | CI behavior |
|---|---|---|
| R0 | Planned only | Missing package manifests may skip. |
| R1 | Skeleton created | Owned directories and minimal docs exist. |
| R2 | First real package manifest exists | CI must run real smoke tests for that package. |
| R3 | Public API exists | Conformance fixtures are required. |
| R4 | Release candidate | Full quality, docs, package, SBOM, provenance, and red-team gates are required. |

## Current state

| Area | Readiness | Evidence |
|---|---|---| 
| Rust workspace | R2 | Root `Cargo.toml` and initial crates exist. |
| Core scheduler/types/state/RNG | R2 | `crates/kairo-ecs-*` skeleton crates compile as the first Track 01 slice. |
| Conformance fixtures | R1 | Initial JSON fixtures exist under `conformance/fixtures`. |
| Binding directories | R1 | Directories exist with README guards; package manifests are intentionally absent. |
| Docs site | R4 | `website` builds the Astro/Starlight docs site with versioning, polyglot plugin, link validation, and desktop/mobile smoke checks. |
| GitHub automation surface | R2 | `.github/` workflows, CODEOWNERS, dependency review, and release scaffolding exist. |
| Packaging | R1 | `packaging/README.md` exists; ecosystem package dirs wait for manifests. |
| FFI bridge (Track 02) | R2 | `crates/kairo-ecs-ffi`, `kairo-ecs-uniffi`, `kairo-ecs-diplomat`, and `include/kairo_ecs.h` exist with lifecycle, panic-boundary, and header-diff compile gates. |
| DES/ABM flow API (Track 03) | R2 | `crates/kairo-ecs-des`, `crates/kairo-ecs-abm`, and `examples/flow` exist with deterministic DES/ABM smoke coverage. |
| Arrow telemetry (Track 04) | R2 | `crates/kairo-ecs-arrow`, `schemas/arrow`, and `examples/telemetry` exist with versioned event-log schema and roundtrip smoke coverage. |
| Visualization (Track 05) | R2 | `crates/kairo-ecs-viz`, `examples/viz`, and `website/docs/visualization` exist with headless-safe optional renderer contracts. |
| Python binding (Track 06) | R2 | `bindings/python` exposes deterministic scheduler, event, Arrow roundtrip, and explicit native-FFI status facades with pytest coverage. |
| R binding (Track 07) | R2 | `bindings/r` exposes deterministic scheduler, event-log, and explicit native-FFI status facades with package docs and static checks. |
| Julia binding (Track 08) | R2 | `bindings/julia` exposes deterministic event ordering, Arrow schema, and explicit native-FFI status facades with package tests. |
| TypeScript/Wasm binding (Track 09) | R2 | `bindings/typescript` and `crates/kairo-ecs-wasm` expose scheduler, event-log, and browser-safe wasm loading contracts with npm and cargo checks. |
| C# binding (Track 10) | R2 | `bindings/csharp` exposes deterministic scheduler/event and native status facades with .NET 10 test/build/pack validation. |
| Go binding (Track 11) | R2 | `bindings/go` exposes deterministic scheduler/event and explicit cgo/native not-configured behavior with `go test` and `go vet` validation. |
| Conformance, testing, and benchmarks (Track 12) | R3 | Reusable conformance runner, ready-fixture validation, and metadata-only benchmark smoke harness exist under `tests/conformance` and `benches`. |
| CI/CD, code quality, and supply chain (Track 13) | R2 | Workflow policy checks, CI skip guard, dependency update policy, and supply-chain workflow gates exist under `.github` and `deny.toml`. |
| Documentation site and education (Track 14) | R2 | `website` builds source-backed docs and runs a local link-check manifest across implemented tracks and binding surfaces. |
| Packaging, publishing, and delivery (Track 15) | R2 | `packaging/release-package-manifest.json` and its builder validate dry-run package inventory and checksum manifest generation. |
| Release governance and maintenance (Track 16) | R2 | Changelog, compatibility, release governance, and maintenance handoff docs exist under `docs/release` and `conductor/maintenance-governance.md`. |
| Community adoption, education, and ecosystem (Track 17) | R2 | Community onboarding, adoption, governance, roadmap, and model-zoo docs exist with a concrete onboarding-docs gate. |
| Comparative benchmarks and reproducibility (Track 18) | R2 | `docs/benchmarks/reproduce-comparison.md` and `benches/benchmark_reproducibility.py` tie comparison claims to ready fixtures and metadata smoke checks. |
| Research software, citation, and archival (Track 19) | R2 | `.zenodo.json`, `docs/research/citation.md`, paper metadata, and a Track 19 validator enforce repository, version, and no-fake-DOI boundaries. |
| OpenSSF, supply-chain trust, and institutional readiness (Track 20) | R2 | `conductor/tracks/20-openssf-supply-chain-institutional-trust/supply-chain-plan.md`, delivery readiness, and quality gates define staged trust evidence and exceptions. |
| Verification, validation, and uncertainty (Track 21) | R2 | `docs/validation/factory-bottleneck-v1-vvuq-note.md` and `scripts/validation/validate-vvuq-note.mjs` validate fixture-backed VVUQ claims for `factory_bottleneck_v1`. |
| Experiment runner and scenario management (Track 22) | R2 | `crates/kairo-ecs-cli`, `examples/experiments`, `scenarios/manifest-index.json`, and `scripts/scenarios/validate-track22-smoke.ps1` exist with scenario manifest and replay validation smoke coverage. |
| Domain starter kits and model zoo (Track 23) | R2 | `examples/starter-kits/starter-kits.yaml`, `docs/model-zoo/inventory.md`, and `examples/model-zoo/validate-inventory.ps1` define and validate the first starter-kit inventory. |
| Playground, demos, and visualization UX (Track 24) | R2 | `website/playground`, `docs/playground/headless-snapshot.md`, and `website/scripts/smoke-playground.mjs` provide a fixture-backed static demo smoke path. |
| API design review and compatibility governance (Track 25) | R2 | `docs/design/protected-surface-inventory.json`, `docs/design/compatibility-governance.md`, and `docs/design/validate-compatibility-pack.ps1` define protected surfaces and release compatibility checks. |
| Interoperability standards review (Track 26) | R2 | `docs/interoperability/standards-review.md` and the Track 26 standards validator map DEVS, FMI/FMU, SBML, CellML, OpenTelemetry, Arrow, and Parquet claims. |
| Developer experience and reproducible environments (Track 27) | R2 | `justfile`, `docs/developer-experience/docs-workflow.md`, and `scripts/dx/validate-docs-workflow.mjs` define and validate the docs bootstrap/build/dev flow. |
| Red team and devil's advocate review (Track 28) | R2 | `reviews/red-team-report.md` and `claim-capability-ledger.json` track release claims, evidence, owners, and blocker classes. |
| Wave manager and execution gatekeeper (Track 29) | R2 | `conductor/wave-policy.md`, `conductor/gates`, and `validate-wave-gates.ps1` derive waves, dependency closure, and critical-path blockers from `tracks.yaml`. |
| Toolchain and version support matrix (Track 30) | R2 | `conductor/toolchain-matrix.md`, `.github/workflows/toolchain-check.yml`, and `validate-toolchain-matrix.ps1` define supported ecosystems, version-drop policy, and CI install checks. |
| Performance regression guard (Track 31) | R2 | `conductor/performance-thresholds.md`, `benches/regression/compare.py`, and `.github/workflows/bench-regression.yml` define benchmark thresholds and regression comparison gates. |
| GPU Compute (Track 32) | R2 | Archived as `Done` for the legacy optional GPU scaffold: `crates/kairo-ecs-gpu` is wired into the workspace with explicit unavailable-backend contracts, feature compile gates, CPU/reference parity harnesses, and persistent-memory contract tests. Real native GPU execution, hardware parity, and benchmark evidence remain release-gated by Track 52 and the HPC parity wave, not claimed by this archive. |
| WebGPU Compute (Track 33) | R2 | Archived as `Done` for the legacy browser WebGPU scaffold: `crates/kairo-ecs-webgpu` and `website/webgpu-demo` provide dependency-free unavailable-backend contracts, CPU/reference parity checks, demo smoke, and WGSL subset validation. Real browser WebGPU device execution, Wasm binding proof, frame-rate evidence, and cross-browser smoke remain release-gated follow-up scope, not claimed by this archive. |
| PDES & Parallel Execution (Track 34) | R2 | Archived as `Done` for the legacy conservative PDES scaffold: `crates/kairo-ecs-pdes` provides dependency-free LP/event/null-message/GVT contracts, topology and transport validation, sequential-vs-partitioned parity fixtures, 4/8/16/32 LP scaling-smoke samples, deadlock stress, and Time Warp spike documentation. Production scheduler integration, wall-clock speedup, distributed runtime proof, and production Time Warp remain release-gated by Tracks 47/48/49/55. |
| Distributed Simulation (Track 35) | R2 | Archived as `Done` for the legacy dependency-free distributed transport scaffold: `crates/kairo-ecs-mpi` and `crates/kairo-ecs-grpc` provide local MPI/gRPC protocol emulators, source/destination envelope checks, pending-event GVT checks, launch-contract guards, migration/telemetry contract proofs, heartbeat/failure classification, and explicit no-real-runtime claims. Real `rsmpi`, `tonic`/`prost`, multi-node execution, migration runtime, distributed telemetry aggregation, and fault tolerance remain release-gated by Tracks 49/54/55. |
| Streaming & Real-Time (Track 36) | R2 | `crates/kairo-ecs-streaming` exists with event-log contract validation and feature compile gates. |
| ML/AI Integration (Track 37) | R2 | `crates/kairo-ecs-ml`, `examples/ml-surrogate`, and `python/kairo_gym` exist with metadata, shape, and action validation. |
| FMI/FMU Digital Twin (Track 38) | R2 | `crates/kairo-ecs-fmi` and `examples/fmi-co-simulation` exist with unpacked FMU, modelDescription, and AAS validation. |
| Cloud/HPC Batch Runners (Track 39) | R2 | Archived as `Done` for the legacy offline cloud/HPC scaffold: `cloud`, `docker`, `k8s`, `hpc/slurm`, and `docs/cloud-hpc` provide manifest/template validation, local Kubernetes render checks, Slurm/static shell policy checks, telemetry checksum sidecars, runtime evidence boundaries, and partial Azure Batch CPU substrate evidence. Real Docker image execution, Kubernetes reconciliation, Slurm scheduler execution, KairoECS cloud batch scenario runs, output/checksum proof, GPU/HPC proof, and AWS/GCP/Azure production canaries remain release-gated by Tracks 43/54/55. |
| Time-Travel Debugging (Track 40) | R2 | `crates/kairo-ecs-debug` and `website/time-travel-demo` exist with trace-line validation and a Node demo smoke harness. |
| Documentation Platform, Quality Gates, and Learning Coverage (Track 41) | R2 | Current docs, CI, example, and notebook surfaces are validated with a learning-coverage matrix, notebook inventory checks, docs workflow smoke, and an explicit docs-platform parity boundary for the active Astro/Starlight site. |
| Cloud/HPC Registry Publication (Track 43) | R2 | Archived as `Done` for the guarded repo-side HPC registry publication and runtime-evidence gate: manifest/workflow/helper/evidence validator, negative production-claim fixture, offline validation, and protected-publish blocker are validated. Live Docker, Kubernetes, Slurm, AWS/GCP Batch, Azure KairoECS container/scenario proof, protected publication, and release-manager approval remain incomplete and are not claimed by this archive. |
| Code and Repository Health (Track 44) | R2 | Archived as `Done` for the repo-side 9.5/10 health floor: scorecard, validator, workflow, and publication dependency wiring are validated at 10/10. Live GitHub API score ingestion, release trend storage, and package-specific sub-scores remain follow-up scope. |
| Astro/Starlight Docs Platform and Polyglot Experience (Track 45) | R2 | Archived as `Done` for the active Astro/Starlight docs platform: versioning, local polyglot metadata, link validation, llms.txt, icons, Pagefind output, docs workflow smoke, and the SOTA validator pass locally. TypeDoc, OpenAPI, and hosted DocSearch remain deferred activation-condition work, not current claims. |
| HPC Parity Charter (Track 46) | R2 | Archived as `Done` for the governance-only HPC parity charter: evidence contract schema, scaffold/live-HPC template manifests, baseline claim boundaries, and executable claim-boundary validator are validated. No runtime HPC capability, live hardware proof, or production parity claim is made by this archive; those remain owned by Tracks 47-55. |
| PDES Conservative Production Runtime (Track 47) | R2 | Done for the event-owned single-host runtime: 18 core Scheduler parity cases, 14 protocol regressions, 8-LP 10,000-tick stress, full native CI and immutable five-repeat CPU profiles accepted. All 19 hosted workflow runs pass at 8a6bf66, including Package Dry Runs under human-approved EXC-193 with raw exit 1 retained. Final metadata-head checks/merge remain required; no speedup or distributed certification, RC/1.0 or publication acceptance is claimed. |
| Time Warp Optimistic Runtime (Track 48) | R2 | Preview event-owned `OptimisticRuntime` and `GenerationBitset` implement replay, exact-incarnation downstream cancellation, bounded execution, typed failure, full-key ordering and GVT safety. Explicitly bound Rust1.98.1 workspace CI passed 458 tests at7a432ab; prior pinned-toolchain labels were withdrawn after actual1.99 compiler verification. Actual1.76 PDES94 tests and compiler-bound68b8d7a benchmark accepted; hosted/security gates and live Track49 distributed acceptance remain pending; Track48 stays In Progress and does not authorize Track49 production dispatch. |
| Distributed MPI/gRPC State Sync (Track 49) | R2 | Implemented `MpiProcessTransport` (`crates/kairo-ecs-mpi/src/process.rs`), `GrpcSocketTransport` (`crates/kairo-ecs-grpc/src/socket.rs`), `KairoOptimisticEnvelopeV1` (`envelope.rs`), `multirank_smoke` (verified across 4 ranks via `mpiexec -n 4`), and `two_process_smoke` (real child OS process TCP socket exchange). Multi-node `live-hpc` cluster evidence and upstream `rsmpi`/`tonic` integration remain gated by Tracks 54/55. |
| NUMA and Memory Lifecycle (Track 50) | R2 | Implemented `NumaTopology::discover()` (sysfs/env discovery with typed fallback), `EventArena` chunked bump allocation, lock-free `EventPool` reuse, multithreaded contention tests, and FFI zero-copy layout validation in `kairo-ecs-core`/`kairo-ecs-state`/`kairo-ecs-ffi`. Multi-socket hardware affinity proof remains gated by Track 55. |
| Parallel I/O and Checkpoint Export (Track 51) | R2 | Implemented `parallel-io`, `hdf5`, and `adios2` feature-gated checkpoint/restart writers, readers, contiguous block manifests, and SHA-256 checksum verification in `kairo-ecs-arrow`. Official upstream `arrow-*`/`parquet` crate integration is scheduled in Track 62 and live Lustre/GPFS evidence remains gated by Track 55. |
| Native GPU Persistent Device Runtime (Track 52) | R2 | Implemented `PersistentGpuSession` (`run_abm_tick_parallel`), Apple Silicon `MetalBackend` (`metal-backend` feature with unified-memory zero-copy borrow, MSL `abm_step` compute kernel source, and multi-core OS thread workgroup execution), `WgpuBackend`, and `CudaBackend` in `kairo-ecs-gpu`. Dedicated multi-GPU NVIDIA CUDA hardware cluster benchmarks remain gated by Track 55. |
| FMI Co-Simulation Master Runtime (Track 53) | R1 | First `fmi-runtime` contract slice adds dependency-free stored-entry `.fmu` archive extraction, path traversal rejection, malformed archive validation, and layout reuse through `FmuArchive::extract_to` in `kairo-ecs-fmi`. Dynamic library loading, FMI 2/3 lifecycle binding, 1,000-step co-simulation, OpenModelica roundtrip, and live FMU evidence remain incomplete. |
| Slurm/Container/Cloud Runtime Acceptance (Track 54) | R1 | Archived as `Done` only for the repo-side blocked-scope runtime-evidence gate: `validate-hpc-runtime-evidence.mjs`, blocked Docker/Kubernetes/Slurm/AWS/GCP/Azure evidence scopes, and the negative missing-checksum fixture validate locally. Live container, scheduler, provider, MPI, and GPU runtime acceptance remain incomplete and are not claimed by this archive. |
| Weak/Strong Scaling Certification (Track 55) | R1 | First certification-contract slice defines representative scenario coverage, draft weak/strong evidence profiles, raw-result checksum policy, and `validate-hpc-scaling-certification.mjs`; the validator now enforces accepted raw-result schemes and required scenario-category coverage. Live scaling proof remains blocked on Tracks 47-54. |
| Game Theory Ontology Wave Charter (Track 56) | R3 | Done after the wave charter, concrete evidence manifests (`conductor/game-theory-evidence/manifests/track-56.json`..`track-61.json`), claim-boundary validator, negative fixtures, and downstream Tracks 57-61 all completed verification. |
| Open Game Theory Ontology Ingestion (Track 57) | R3 | Done with `open-game-theory-ontology/`, Turtle and JSON-LD fixtures, `kairo-ecs-game-ontology` parser crate, canonical IR normalization, and concrete manifest evidence. |
| Ontology-to-Rust Code Generation (Track 58) | R3 | Done with deterministic ontology-to-Rust component codegen, golden fixture verification, `generated-components` integration in `kairo-ecs-game-theory`, and concrete manifest evidence. |
| Graph Relations ECS Module (Track 59) | R3 | Done with feature-gated `graph-relations`, Entity-ID `ChildOf`/`TransitionTo` components, cycle-safe flat-store traversal helpers, pointer-topology scanner, and concrete manifest evidence. |
| Normal-Form Multi-Game Runtime (Track 60) | R3 | Done with normal-form components, best-response, pure Nash, strict dominance, Lemke-Howson mixed-strategy Nash, support enumeration, correlated equilibrium, replicator dynamics, benchmark smoke, and concrete manifest evidence. |
| Extensive-Form Graph-ECS Runtime (Track 61) | R3 | Done with extensive-form Graph-ECS components, topology validation, backward induction, chance-weighted expected-utility solver (`ChanceWeightedBackwardInductionSolver`), `CfrPlusSolver`, and multi-game certification manifests. |
| Official Arrow/Parquet & Zero-Copy FFI Spine (Track 62) | R1 | Done Wave 9 track to replace custom TSV/binary smoke envelopes with official `arrow-*` and `parquet` crates and export the `FFI_ArrowArray`/`FFI_ArrowSchema` zero-copy C Data Interface. |
| Native Polyglot Bindings (Track 63) | R1 | Done Wave 9 track to replace pure-language fallback schedulers with compiled `PyO3`/`maturin` wheels, `wasm-bindgen` modules, and C ABI dynamic libraries across Python, TypeScript, R, Julia, C#, and Go. |
| SoA ECS, DES Resource Kernel & ABM Spatial Topologies (Track 64) | R1 | Done Wave 9 track to upgrade `kairo-ecs-state` to 64-byte aligned SoA columns and add SimPy-grade DES resources/processes and Agents.jl-grade ABM grid/continuous/graph spaces. |
| Vectorized MARL (`VecEnv`) & Advanced Game Solvers (Track 65) | R1 | Done Wave 9 track for Mixed-Strategy Nash (Lemke-Howson), Correlated Equilibrium, `CFR`/`CFR+`, Replicator Dynamics, PettingZoo adapters, and Rayon-parallel `VecEnv` stepping. |
| Ensemble Sweeps (Sobol/CRN), SBI Calibration & CoW Wasm Studio (Track 66) | R1 | Done Wave 9 track for Sobol/LHS + Common Random Numbers ensemble sweeps, Optuna/ABC-SMC calibration hooks, Copy-on-Write delta snapshots, and live Wasm browser time-travel debugging. |

## Enforcement rule

Once a track moves to `In Progress`, the files listed in `conductor/tracks.yaml` for that track must exist or be explicitly waived in the track handoff. Once an ecosystem package manifest is added, matching CI must fail on errors rather than skip.

Tracks cannot move to `In Review` or `Done` from planning text alone. A closeout must identify the owned files that exist in the worktree, the commands that exercised each required gate, and any waived gate with an owner and follow-up. R2 means a real checked-in implementation slice exists and compiles or validates locally; it does not imply release-candidate completeness.

## Immediate critical path

1. Track 01: complete `lanes.md` milestones for types, scheduler, state, RNG, and facade readiness.
2. Track 12: turn JSON fixtures into a shared runner.
3. Track 13: add a metadata validator for `conductor/tracks.yaml` and keep the existing GitHub workflows aligned with track metadata.
4. Track 14: keep the Astro/Starlight docs stack, versioning, polyglot plugin, and smoke checks aligned with release evidence.
5. Track 15: keep the first registry/package dry-run sequence aligned with the new Track 42 publication gate before any public write.

Track 00 is closed as `Done` after maintainer approval of the foundation naming evidence on 2026-05-07. Later release tracks still own production publishing, package dry-runs, SBOM, provenance, and registry-specific publish gates.

## Approved conditional Track49 entry — 4 October2026

The human sole maintainer explicitly approved Track49 production scheduling after scope clarification. See conductor/tracks/29-wave-manager-execution-gatekeeper/adr-track49-phase-entry-20261004.md. Dispatch remains conditional on normal PR199 merge, exact-head gates/security evidence, accepted local/interface proof and a separately reviewed reserved transport packet. Track48 remains In Progress; dependencies, full distributed acceptance and release gates are unchanged. The raw Track49 gate result is preserved, not rewritten as a pass.

## Track 70 readiness

[Track 70](tracks/70-polars-native-data-processing/index.md) has completed Phase 0 qualification and Phase 1 native tables, advancing to Phase 2 (FFI and polyglot consumers). Dependencies 04/62/63 are routing context, not runtime proof. Complete upstream Phase 4 before CareOps `polars_data_processing_20261022` adopts the pin.

## Library adoption planning — 22 October 2026

[Programme](library-adoption.md): Tracks 71 (rand), 72 (rand_distr) and 73 (petgraph) are Planned, implementation not started. ECharts and Three.js are separate CareOps tracks. Entry requires explicit cross-repository acceptance receipts; both Polars migrations come first. No current runtime or release gate is waived.

## Robustness-first dependency registration — 22 October 2026

[Tracks 74–92](robust-dependencies.md) are Planned; priority/evaluation/conditional classes are explicit. No implementation, installation or acceptance. Foundation gates 74–77 precede rand; Rayon 78 precedes CareOps ECharts through a manual receipt gate. Optional candidates do not block core work. SUMO remains CareOps-conditional; Rapier deferred.

## Dependency sequencing refinement

See [current sequencing](dependency-sequencing.md): Polars inventory first, with sha2/tempfile/proptest gated only at demonstrated dependent tasks; thiserror after CLI foundation and before RNG; diagnostics/concurrency/state-machine additions conditional. No implementation acceptance.

## Validation/tooling refinement

[Sequencing](dependency-sequencing.md) registers Criterion reuse, schema-validation gates and conditional ODE/WebSocket work. No runtime acceptance.

## Python runtime qualification — 23 October 2026

[Python 3.15 migration and qualification](tracks/105-python315-runtime-qualification/index.md) is Planned for accepted migration. Existing local candidate changes and test evidence are preserved; no main/runtime handover acceptance or unrelated phase change. Track 30 retains version policy; bounded shared-path handoffs precede implementation.
