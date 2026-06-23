# KairoECS Implementation Readiness

Last verified: 2026-06-19

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
| GPU Compute (Track 32) | R2 | `crates/kairo-ecs-gpu` exists, is wired into the root workspace, and has explicit unavailable-backend contracts plus feature compile gates; the GNU-toolchain runtime rerun now passes, but real GPU parity and benchmark evidence still need hardware. |
| WebGPU Compute (Track 33) | R2 | `crates/kairo-ecs-webgpu` and `website/webgpu-demo` exist with browser dispatch capability checks and demo smoke tests; the GNU-toolchain runtime rerun now passes, but browser WebGPU device proof still needs a browser-capable runtime. |
| PDES & Parallel Execution (Track 34) | R2 | `crates/kairo-ecs-pdes` exists with parity and deadlock-stress report fixtures; the GNU-toolchain runtime rerun now passes, but scaling benchmarks and Time Warp evidence remain pending. |
| Distributed Simulation (Track 35) | R2 | `crates/kairo-ecs-mpi` and `crates/kairo-ecs-grpc` exist with dependency-free transport protocol emulators and compile gates; the GNU-toolchain runtime rerun now passes, but real `rsmpi`/`tonic` and multi-node evidence remain pending. |
| Streaming & Real-Time (Track 36) | R2 | `crates/kairo-ecs-streaming` exists with event-log contract validation and feature compile gates. |
| ML/AI Integration (Track 37) | R2 | `crates/kairo-ecs-ml`, `examples/ml-surrogate`, and `python/kairo_gym` exist with metadata, shape, and action validation. |
| FMI/FMU Digital Twin (Track 38) | R2 | `crates/kairo-ecs-fmi` and `examples/fmi-co-simulation` exist with unpacked FMU, modelDescription, and AAS validation. |
| Cloud/HPC Batch Runners (Track 39) | R2 | `cloud`, `docker`, `k8s`, `hpc/slurm`, and `docs/cloud-hpc` exist with offline cloud/HPC validation plus a runtime evidence boundary; live Docker, Kubernetes, Slurm, and provider acceptance still need environment-backed proof. |
| Time-Travel Debugging (Track 40) | R2 | `crates/kairo-ecs-debug` and `website/time-travel-demo` exist with trace-line validation and a Node demo smoke harness. |
| Documentation Platform, Quality Gates, and Learning Coverage (Track 41) | R2 | Current docs, CI, example, and notebook surfaces are validated with a learning-coverage matrix, notebook inventory checks, docs workflow smoke, and an explicit docs-platform parity boundary for the active Astro/Starlight site. |
| Cloud/HPC Registry Publication (Track 43) | R2 | In Review after the guarded HPC registry workflow, publication manifest, runtime acceptance evidence manifest, negative production-claim fixture, and readiness validator gate. Live Docker, Kubernetes, Slurm, AWS/GCP Batch, Azure KairoECS container/scenario proof, protected publication, and release-manager approval remain incomplete. |
| Astro/Starlight Docs Platform and Polyglot Experience (Track 45) | R2 | The active docs platform uses Astro/Starlight with versioning, local polyglot metadata, link validation, llms.txt, icons, Pagefind output, and a dedicated SOTA validator. |
| HPC Parity Charter (Track 46) | R2 | Evidence contract schema, scaffold/live-HPC template manifests, and executable claim-boundary validator exist. This is governance evidence only; no runtime HPC capability is claimed. |
| PDES Conservative Production Runtime (Track 47) | R2 | Local conservative PDES evidence now includes typed lookahead enforcement, deterministic `PartitionPlan` entity-to-LP assignment and validation, sequential/partitioned parity fixtures, GVT monotonicity, deadlock smoke, and 4/8/16/32 LP local benchmark-smoke samples. Full production scheduler integration, wall-clock benchmarks, Track 46 raw evidence manifests, and live scaling proof remain incomplete. |
| Time Warp Optimistic Runtime (Track 48) | R2 | Local `time-warp` runtime now covers straggler rollback, anti-message cancellation, generation-stale component access guards, monotonic-GVT fossil collection, pre-GVT straggler rejection, duplicate-positive rejection, and overhead/rollback-pressure counters in `kairo-ecs-pdes`. Production optimistic scheduling, replay/redelivery semantics, downstream-output anti-messages, benchmark evidence, distributed transport, and live HPC evidence remain incomplete. |
| Distributed MPI/gRPC State Sync (Track 49) | R1 | First contract-baseline slice adds explicit MPI multi-rank launch descriptors, gRPC two-process launch descriptors, and migration snapshots that preserve entity/component generations plus pending-event metadata. Real `rsmpi`, `tonic`/`prost`, multi-rank launch, and two-process socket evidence remain incomplete. |
| NUMA and Memory Lifecycle (Track 50) | R1 | First `numa` feature-gated contract slice adds typed unsupported topology metadata, opt-in reversible affinity plans, a dependency-free event pool reuse contract, component locality metadata, and FFI zero-copy layout validation. Real `hwloc`, OS affinity binding, production arenas, contention evidence, and NUMA hardware proof remain incomplete. |
| Parallel I/O and Checkpoint Export (Track 51) | R1 | First `parallel-io` feature-gated contract slice adds checked event-log batch metadata, contiguous block manifests, checkpoint/restart envelopes, and HDF5/ADIOS2 format contract tests in `kairo-ecs-arrow`. Native Apache Arrow arrays, HDF5/ADIOS2 writers, parallel filesystem evidence, and throughput benchmarks remain incomplete. |
| Native GPU Persistent Device Runtime (Track 52) | R1 | First GPU-contract slice adds a persistent device-memory session surface, resident-buffer lifecycle counters, copy-boundary accounting, and deterministic ABM/DES CPU parity tests in `kairo-ecs-gpu`. Real `wgpu`/CUDA device initialization, shader/kernel dispatch, hardware benchmarks, and live GPU evidence remain incomplete. |
| FMI Co-Simulation Master Runtime (Track 53) | R1 | First `fmi-runtime` contract slice adds dependency-free stored-entry `.fmu` archive extraction, path traversal rejection, malformed archive validation, and layout reuse through `FmuArchive::extract_to` in `kairo-ecs-fmi`. Dynamic library loading, FMI 2/3 lifecycle binding, 1,000-step co-simulation, OpenModelica roundtrip, and live FMU evidence remain incomplete. |
| Slurm/Container/Cloud Runtime Acceptance (Track 54) | R1 | First runtime-evidence gate slice adds `validate-hpc-runtime-evidence.mjs`, a blocked-scope evidence manifest for Docker, Kubernetes, Slurm, AWS Batch, GCP Batch, and Azure Batch, and a negative missing-checksum fixture. Live container, scheduler, and provider canaries remain incomplete. |
| Weak/Strong Scaling Certification (Track 55) | R1 | First certification-contract slice defines representative scenario coverage, draft weak/strong evidence profiles, raw-result checksum policy, and `validate-hpc-scaling-certification.mjs`; the validator now enforces accepted raw-result schemes and required scenario-category coverage. Live scaling proof remains blocked on Tracks 47-54. |
| Game Theory Ontology Wave Charter (Track 56) | R1 | In Review after the wave charter, evidence schema/templates, claim-boundary validator, negative fixtures, release-boundary spec sections, and closeout handoff were reconciled; Track 56 remains governance-only and cannot move Done until downstream Tracks 57-61 evidence is complete or excluded from release claims. |
| Open Game Theory Ontology Ingestion (Track 57) | R2 | In Review after `open-game-theory-ontology/`, Turtle and JSON-LD fixtures, parser tests, canonical IR normalization, review fixes, release-boundary spec sections, and closeout handoff were reconciled; broad external RDF/OWL/JSON-LD compatibility remains future scope. |
| Ontology-to-Rust Code Generation (Track 58) | R2 | Phase 1 is In Review after deterministic ontology-to-Rust generation, generated component compile gates, API/review fixes, release-package manifest coverage, push, strict closeout, and GitHub Actions review. |
| Graph Relations ECS Module (Track 59) | R2 | Phase 1 is In Review: `graph-relations` is default-off, Entity-ID `ChildOf`/`TransitionTo` components and flat-store traversal helpers are implemented, pointer-topology scan passes, and GitHub Actions passed after rerunning a transient crates.io fetch failure. |
| Normal-Form Multi-Game Runtime (Track 60) | R2 | Phase 2 is In Review after normal-form components, best-response, pure Nash, strict dominance, quick benchmark smoke, runnable example, bounded docs, phase push, strict git closeout, and GitHub Actions review passed. Broader benchmark evidence, mixed-strategy solving, generated-ontology integration, and release-parity evidence remain follow-up scope. |
| Extensive-Form Graph-ECS Runtime (Track 61) | R3 | Phase 2 local multi-game certification is implemented across normal-form flat arrays and extensive-form Graph-ECS stores; push, strict git closeout, and GitHub Actions review passed. The track is In Review pending any accepted stochastic extensive-form and release-governed follow-up evidence before Done. |

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
