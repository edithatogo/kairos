# KairoECS Tracks Index

This file is the human-readable Conductor status index. Keep it in lockstep with `conductor/tracks.yaml`, which is the authoritative machine-readable source for status, ownership, dependencies, paths, and gates.

| Track | Name | Status | Primary subagent | Artifact |
|---:|---|---|---|---|
| 00 | Project Foundation, Governance & Naming | Done | `foundation-agent` | See `conductor/tracks/00-*/spec.md` |
| 01 | The Heart: kairo-ecs-core & kairo-ecs-state | Done | `core-scheduler-agent + ecs-agent + contracts-agent` | See `conductor/tracks/01-*/spec.md` |
| 02 | The Bridge: kairo-ecs-ffi, UniFFI & Diplomat | Done | `ffi-agent + uniffi-agent + diplomat-agent` | See `conductor/tracks/02-*/spec.md` |
| 03 | The Flow: DES Trajectory API & ABM Behavior API | Done | `des-api-agent + abm-api-agent` | See `conductor/tracks/03-*/spec.md` |
| 04 | The Analyst: kairo-ecs-arrow | Done | `arrow-agent` | See `conductor/tracks/04-*/spec.md` |
| 05 | The Window: kairo-ecs-viz | Done | `viz-agent` | See `conductor/tracks/05-*/spec.md` |
| 06 | Python Binding 3.10-3.14 | Done | `python-agent` | See `conductor/tracks/06-*/spec.md` |
| 07 | R Binding | Done | `r-agent` | See `conductor/tracks/07-*/spec.md` |
| 08 | Julia Binding | Done | `julia-agent` | See `conductor/tracks/08-*/spec.md` |
| 09 | TypeScript/Wasm Binding | Done | `typescript-agent` | See `conductor/tracks/09-*/spec.md` |
| 10 | C# Binding .NET 10-11 | Done | `csharp-agent` | See `conductor/tracks/10-*/spec.md` |
| 11 | Go Binding | Done | `go-agent` | See `conductor/tracks/11-*/spec.md` |
| 12 | Conformance, Testing & Benchmarks | Done | `conformance-agent + performance-agent` | See `conductor/tracks/12-*/spec.md` |
| 13 | CI/CD, Code Quality & Supply Chain | Done | `ci-agent + security-agent` | See `conductor/tracks/13-*/spec.md` |
| 14 | Documentation Site & Education | Done | `docs-agent` | See `conductor/tracks/14-*/spec.md` |
| 15 | Packaging, Publishing & Delivery | Done | `release-agent` | See `conductor/tracks/15-*/spec.md` |
| 16 | Release Governance & Maintenance | Done | `release-agent + governance-agent` | See `conductor/tracks/16-*/spec.md` |
| 17 | Community Adoption, Education & Ecosystem | Done | `community-agent` | See `conductor/tracks/17-*/spec.md` |
| 18 | Comparative Benchmarks & Reproducibility | Done | `benchmark-agent` | See `conductor/tracks/18-*/spec.md` |
| 19 | Research Software, Citation & Archival | Done | `research-agent` | See `conductor/tracks/19-*/spec.md` |
| 20 | OpenSSF, Supply Chain Trust & Institutional Readiness | Done | `security-agent` | See `conductor/tracks/20-*/spec.md` |
| 21 | Verification, Validation & Uncertainty | Done | `vv-uq-agent` | See `conductor/tracks/21-*/spec.md` |
| 22 | Experiment Runner & Scenario Management | Done | `experiment-agent` | See `conductor/tracks/22-*/spec.md` |
| 23 | Domain Starter Kits & Model Zoo | Done | `model-zoo-agent` | See `conductor/tracks/23-*/spec.md` |
| 24 | Playground, Demos & Visualization UX | Done | `playground-agent` | See `conductor/tracks/24-*/spec.md` |
| 25 | API Design Review & Compatibility Governance | Done | `api-governance-agent` | See `conductor/tracks/25-*/spec.md` |
| 26 | Interoperability Standards Review | Done | `interop-agent` | See `conductor/tracks/26-*/spec.md` |
| 27 | Developer Experience & Reproducible Environments | Done | `dx-agent` | See `conductor/tracks/27-*/spec.md` |
| 28 | Red Team & Devil's Advocate Review | Done | `redteam-agent` | See `conductor/tracks/28-*/spec.md` |
| 29 | Wave Manager & Execution Gatekeeper | Done | `wave-manager-agent` | See `conductor/tracks/29-*/spec.md` |
| 30 | Toolchain & Version Support Matrix | Done | `toolchain-agent` | See `conductor/tracks/30-*/spec.md` |
| 31 | Performance Regression Guard | Done | `perf-regression-agent` | See `conductor/tracks/31-*/spec.md` |
| 32 | GPU Compute Acceleration | Done | `gpu-compute-agent` | See `conductor/tracks/32-*/spec.md` |
| 33 | WebGPU Compute for Browser | Done | `webgpu-agent` | See `conductor/tracks/33-*/spec.md` |
| 34 | PDES & Parallel Execution | Done | `pdes-agent` | See `conductor/tracks/34-*/spec.md` |
| 35 | Distributed Simulation (MPI/gRPC) | Done | `distributed-agent` | See `conductor/tracks/35-*/spec.md` |
| 36 | Streaming & Real-Time Processing | Done | `streaming-agent` | See `conductor/tracks/36-*/spec.md` |
| 37 | ML/AI Integration & Inference | Done | `ml-integration-agent` | See `conductor/tracks/37-*/spec.md` |
| 38 | FMI/FMU & Digital Twin Bridge | Done | `fmi-agent` | See `conductor/tracks/38-*/spec.md` |
| 39 | Cloud/HPC Batch Runners | Done | `cloud-agent` | See `conductor/tracks/39-*/spec.md` |
| 40 | Time-Travel Debugging & Interactive Stepping | Done | `timetravel-agent` | See `conductor/tracks/40-*/spec.md` |
| 41 | Documentation Platform, Quality Gates & Learning Coverage | Done | `docs-agent + ci-agent + community-agent` | See `conductor/tracks/41-*/spec.md` |
| 42 | Package Registry Publication & Provenance | Done | `publication-agent + release-agent + binding agents` | Archived repo-side guarded publication/provenance gate; live registry publication remains release-gated. |
| 43 | Cloud/HPC Registry Publication & Runtime Acceptance | Done | `hpc-registry-agent + cloud-agent + release-agent` | See `conductor/tracks/43-*/spec.md` |
| 44 | Code and Repository Health >= 9.5 | Done | `health-agent + ci-agent + security-agent + release-agent` | See `conductor/tracks/44-*/spec.md` |
| 45 | Astro/Starlight Docs Platform and Polyglot Experience | Done | `docs-platform-agent + docs-agent + ci-agent` | See `conductor/tracks/45-*/spec.md` |
| 46 | HPC Parity Charter, Baselines & Evidence Gates | Done | `benchmark-agent + interop-agent + redteam-agent + wave-manager-agent` | See `conductor/tracks/46-*/spec.md` |
| 47 | PDES Conservative Lookahead Production Runtime | Done | `pdes-agent + performance-agent` | See `conductor/tracks/47-*/spec.md` |
| 48 | Time Warp Optimistic Rollback Runtime | In Progress | `pdes-agent + timetravel-agent + ecs-agent` | Local runtime reviewed; compiler-bound evidence accepted; hosted/distributed gates pending. See `conductor/tracks/48-*/handoff.md`. |
| 49 | Distributed MPI/gRPC State Synchronization | In Progress | `distributed-agent + pdes-agent` | See `conductor/tracks/49-*/spec.md` |
| 50 | NUMA Topology, Affinity & HPC Memory Lifecycle | In Progress | `core-scheduler-agent + ecs-agent + ffi-agent` | See `conductor/tracks/50-*/spec.md` |
| 51 | Parallel I/O, Arrow Record Batches & Checkpoint Export | In Progress | `arrow-agent + cloud-agent + performance-agent` | See `conductor/tracks/51-*/spec.md` |
| 52 | Native GPU Acceleration with Persistent Device Memory | In Progress | `gpu-compute-agent + performance-agent` | See `conductor/tracks/52-*/spec.md` |
| 53 | FMI 2/3 Co-Simulation Master Runtime | In Progress | `fmi-agent + ffi-agent + interop-agent` | See `conductor/tracks/53-*/spec.md` |
| 54 | Slurm, Container & Cloud HPC Runtime Acceptance | Done | `cloud-agent + distributed-agent + gpu-compute-agent + release-agent` | See `conductor/tracks/54-*/spec.md` |
| 55 | End-to-End Weak/Strong Scaling Certification | In Progress | `performance-agent + benchmark-agent + cloud-agent + release-agent` | See `conductor/tracks/55-*/spec.md` |
| 56 | Game Theory Ontology Wave Charter and Evidence Gates | Done | `ontology-agent + game-theory-agent + wave-manager-agent` | See `conductor/tracks/56-*/spec.md` |
| 57 | Open Game Theory Ontology Subrepo and Schema Ingestion | Done | `ontology-agent + rust-agent` | See `conductor/tracks/57-*/spec.md` |
| 58 | Ontology-to-Rust Component Code Generation | Done | `ontology-agent + ecs-agent + api-governance-agent` | See `conductor/tracks/58-*/spec.md` |
| 59 | Feature-Gated Graph Relations ECS Module | Done | `ecs-agent + core-scheduler-agent + conformance-agent` | See `conductor/tracks/59-*/spec.md` |
| 60 | Normal-Form Multi-Game Runtime and Solvers | Done | `game-theory-agent + ecs-agent + performance-agent` | See `conductor/tracks/60-*/spec.md` |
| 61 | Extensive-Form Graph-ECS Runtime and Certification | Done | `game-theory-agent + ecs-agent + benchmark-agent + redteam-agent` | See `conductor/tracks/61-*/spec.md` |
| 62 | Official Apache Arrow, Parquet & Zero-Copy C Data Interface Spine | Done | `arrow-agent + ffi-agent + interop-agent` | See `conductor/tracks/62-*/spec.md` |
| 63 | Native Polyglot Bindings (PyO3/Maturin, Wasm-Bindgen, Extendr & C ABI) | Done | `python-agent + typescript-agent + r-agent + julia-agent + csharp-agent + go-agent + ffi-agent` | See `conductor/tracks/63-*/spec.md` |
| 64 | Archetype/SoA ECS Storage, Full DES Process/Resource Kernel & ABM Spatial Topologies | Done | `ecs-agent + des-api-agent + abm-api-agent + core-scheduler-agent` | See `conductor/tracks/64-*/spec.md` |
| 65 | Vectorized Multi-World RL (VecEnv), PettingZoo Bridge & Advanced Game Theory Solvers | Done | `game-theory-agent + ml-integration-agent + performance-agent` | See `conductor/tracks/65-*/spec.md` |
| 66 | Parallel Ensemble Sweeps (Sobol/CRN), SBI Calibration & Copy-on-Write Wasm Studio | Done | `vv-uq-agent + experiment-agent + timetravel-agent + playground-agent` | See `conductor/tracks/66-*/spec.md` |
| 67 | Linked Entity Hierarchies, Dynamic Spawning & Cascaded Lifecycles | Done | `ecs-agent + abm-api-agent + contracts-agent` | See `conductor/tracks/67-*/spec.md` |
| 68 | Inter-Unit Boundary Channels & Asynchronous Handover Sinks | Done | `des-api-agent + contracts-agent + streaming-agent` | See `conductor/tracks/68-*/spec.md` |
| 69 | Stochastic Competing Risks & Non-Markovian Hazard Engine | Done | `des-api-agent + vv-uq-agent + core-scheduler-agent` | See `conductor/tracks/69-*/spec.md` |

## Track status vocabulary

```text
Planned -> Spec Approved -> In Progress -> In Review -> Blocked -> Done -> Deferred -> Cancelled
```

Machine-readable track metadata is maintained in `conductor/tracks.yaml`.

## Release criticality

- Tracks 00, 01, 02, 12, 13, 14, 15, 16, 20, 25, 28, 29, and 30 are release-gating for any public release.
- Track 31 is quality-improving but not release-gating.
- Tracks 42, 43, and 44 are release-gating for any production registry publication.
- Track 45 is release-gating for docs-platform publication claims and public docs release notes.
- Tracks 06 and 09 are recommended v0.1/v0.2 public binding priorities.
- Tracks 07, 08, 10, and 11 can remain preview until C ABI and Arrow schema stability are proven.
- Track 05 must remain optional and non-blocking for headless releases.
- GitHub automation, dependency review, scorecard, SBOM, and release workflows already exist under `.github/`; registry publication manifests are now owned by Track 42, cloud/HPC publication manifests by Track 43, and production publication by the Track 44 health gate.
- Tracks 34, 35, and 38 are release-critical for distributed simulation, FMI digital-twin, and PDES infrastructure.
- Tracks 32, 33, 36, 37, 39, 40, and 41 are non-release-critical (enhancement only).
- Tracks 46-55 are release-gating for any production HPC parity claim. They do not mark existing scaffold tracks complete; they define the live-proof layer for PDES, Time Warp, MPI/gRPC, NUMA, parallel I/O, GPU, FMI, Slurm/cloud runtime acceptance, and weak/strong scaling certification.
- Tracks 56-61 are release-gating for any public open-game-theory ontology, graph-relational ECS, or multi-game solver claim. They require task-level commits, phase review/push closeouts, and GitHub Actions review before any track can move beyond `In Review`.
- Tracks 62-66 define Wave 9 (Native Spine, Ergonomics & Bleeding-Edge Competitor Parity): official Apache Arrow/Parquet/FFI zero-copy integration, compiled native polyglot extensions (`PyO3`, `wasm-bindgen`, C ABI), SoA ECS + SimPy-grade DES resources + Agents.jl-grade ABM spatial topologies, vectorized MARL (`VecEnv`) + CFR+/Mixed-Nash game solvers, and Sobol/CRN ensemble calibration + CoW Wasm time-travel studio.

## Approved conditional Track49 entry — 4 October2026

The human sole maintainer explicitly approved Track49 production scheduling after scope clarification. See conductor/tracks/29-wave-manager-execution-gatekeeper/adr-track49-phase-entry-20261004.md. Dispatch remains conditional on normal PR199 merge, exact-head gates/security evidence, accepted local/interface proof and a separately reviewed reserved transport packet. Track48 remains In Progress; dependencies, full distributed acceptance and release gates are unchanged. The raw Track49 gate result is preserved, not rewritten as a pass.

---
- [ ] **Track 70: Polars-native data processing and standard interchange** — Planned. [Track](tracks/70-polars-native-data-processing/index.md). CareOps follows accepted upstream delivery.

---
- [ ] **Track: rand deterministic RNG adoption** — Planned; implementation not started. [Track](tracks/71-rand-deterministic-rng/index.md).

---
- [ ] **Track: rand_distr statistical sampling adoption** — Planned; implementation not started. [Track](tracks/72-rand-distr-sampling/index.md).

---
- [ ] **Track: petgraph spatial topology and routing adoption** — Planned; implementation not started. [Track](tracks/73-petgraph-spatial-routing/index.md).

---
- [ ] **Track 74: Serde typed control contracts** — Planned; foundation; value gate pending. [Track](tracks/74-serde-robustness/index.md).

---
- [ ] **Track 75: serde_json metadata and control encoding** — Planned; foundation; value gate pending. [Track](tracks/75-serde-json-robustness/index.md).

---
- [ ] **Track 76: toml scenario configuration parsing** — Planned; foundation; value gate pending. [Track](tracks/76-toml-robustness/index.md).

---
- [ ] **Track 77: Clap CLI consolidation** — Planned; foundation; value gate pending. [Track](tracks/77-clap-robustness/index.md).

---
- [ ] **Track 78: Rayon bounded independent-work execution** — Planned; foundation; value gate pending. [Track](tracks/78-rayon-robustness/index.md).

---
- [ ] **Track 79: zip FMU archive replacement** — Planned; subsystem; value gate pending. [Track](tracks/79-zip-robustness/index.md).

---
- [ ] **Track 80: quick-xml FMU metadata replacement** — Planned; subsystem; value gate pending. [Track](tracks/80-quick-xml-robustness/index.md).

---
- [ ] **Track 81: Prost distributed envelope encoding** — Planned; conditional; value gate pending. [Track](tracks/81-prost-robustness/index.md).

---
- [ ] **Track 82: Tokio bounded transport runtime** — Planned; conditional; value gate pending. [Track](tracks/82-tokio-robustness/index.md).

---
- [ ] **Track 83: Tonic interoperable RPC transport** — Planned; conditional; value gate pending. [Track](tracks/83-tonic-robustness/index.md).

---
- [ ] **Track 84: hecs ECS storage qualification** — Planned; evaluation; value gate pending. [Track](tracks/84-hecs-robustness/index.md).

---
- [ ] **Track 85: Clarabel convex solver qualification** — Planned; conditional; value gate pending. [Track](tracks/85-clarabel-robustness/index.md).

---
- [ ] **Track 86: rstar continuous spatial index qualification** — Planned; evaluation; value gate pending. [Track](tracks/86-rstar-robustness/index.md).

---
- [ ] **Track 87: geo geometry and route-position primitives** — Planned; conditional; value gate pending. [Track](tracks/87-geo-robustness/index.md).

---
- [ ] **Track 88: rsmpi native MPI qualification** — Planned; conditional; value gate pending. [Track](tracks/88-mpi-robustness/index.md).

---
- [ ] **Track 89: Tract inference qualification** — Planned; conditional; value gate pending. [Track](tracks/89-tract-robustness/index.md).

---
- [ ] **Track 90: Argmin calibration optimiser qualification** — Planned; conditional; value gate pending. [Track](tracks/90-argmin-robustness/index.md).

---
- [ ] **Track 91: rust-fmi adapter qualification** — Planned; evaluation; value gate pending. [Track](tracks/91-fmi-robustness/index.md).

---
- [ ] **Track 92: Sobol sequence qualification** — Planned; evaluation; value gate pending. [Track](tracks/92-sobol-burley-robustness/index.md).

---
- [ ] **Track 93: sha2 robustness** — foundation; [plan](tracks/93-sha2-integrity/index.md).

---
- [ ] **Track 94: tempfile robustness** — foundation; [plan](tracks/94-tempfile-publication/index.md).

---
- [ ] **Track 95: thiserror robustness** — foundation; [plan](tracks/95-thiserror-contracts/index.md).

---
- [ ] **Track 96: proptest robustness** — foundation; [plan](tracks/96-proptest-invariants/index.md).

---
- [ ] **Track 97: tracing qualification** — conditional; [plan](tracks/97-tracing-diagnostics/index.md).

---
- [ ] **Track 98: loom qualification** — evaluation; [plan](tracks/98-loom-concurrency/index.md).

---
- [ ] **Track 99: proptest-state-machine qualification** — evaluation; [plan](tracks/99-proptest-state-machine/index.md).

---
- [ ] **Track 100: tracing-subscriber qualification** — conditional; [plan](tracks/100-tracing-subscriber/index.md).
