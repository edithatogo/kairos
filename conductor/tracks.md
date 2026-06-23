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
| 42 | Package Registry Publication & Provenance | In Review | `publication-agent + release-agent + binding agents` | See `conductor/tracks/42-*/spec.md` |
| 43 | Cloud/HPC Registry Publication & Runtime Acceptance | Done | `hpc-registry-agent + cloud-agent + release-agent` | See `conductor/tracks/43-*/spec.md` |
| 44 | Code and Repository Health >= 9.5 | Done | `health-agent + ci-agent + security-agent + release-agent` | See `conductor/tracks/44-*/spec.md` |
| 45 | Astro/Starlight Docs Platform and Polyglot Experience | Done | `docs-platform-agent + docs-agent + ci-agent` | See `conductor/tracks/45-*/spec.md` |
| 46 | HPC Parity Charter, Baselines & Evidence Gates | Done | `benchmark-agent + interop-agent + redteam-agent + wave-manager-agent` | See `conductor/tracks/46-*/spec.md` |
| 47 | PDES Conservative Lookahead Production Runtime | In Progress | `pdes-agent + performance-agent` | See `conductor/tracks/47-*/spec.md` |
| 48 | Time Warp Optimistic Rollback Runtime | In Progress | `pdes-agent + timetravel-agent + ecs-agent` | See `conductor/tracks/48-*/spec.md` |
| 49 | Distributed MPI/gRPC State Synchronization | In Progress | `distributed-agent + pdes-agent` | See `conductor/tracks/49-*/spec.md` |
| 50 | NUMA Topology, Affinity & HPC Memory Lifecycle | In Progress | `core-scheduler-agent + ecs-agent + ffi-agent` | See `conductor/tracks/50-*/spec.md` |
| 51 | Parallel I/O, Arrow Record Batches & Checkpoint Export | In Progress | `arrow-agent + cloud-agent + performance-agent` | See `conductor/tracks/51-*/spec.md` |
| 52 | Native GPU Acceleration with Persistent Device Memory | In Progress | `gpu-compute-agent + performance-agent` | See `conductor/tracks/52-*/spec.md` |
| 53 | FMI 2/3 Co-Simulation Master Runtime | In Progress | `fmi-agent + ffi-agent + interop-agent` | See `conductor/tracks/53-*/spec.md` |
| 54 | Slurm, Container & Cloud HPC Runtime Acceptance | In Review | `cloud-agent + distributed-agent + gpu-compute-agent + release-agent` | See `conductor/tracks/54-*/spec.md` |
| 55 | End-to-End Weak/Strong Scaling Certification | In Progress | `performance-agent + benchmark-agent + cloud-agent + release-agent` | See `conductor/tracks/55-*/spec.md` |
| 56 | Game Theory Ontology Wave Charter and Evidence Gates | In Review | `ontology-agent + game-theory-agent + wave-manager-agent` | See `conductor/tracks/56-*/spec.md` |
| 57 | Open Game Theory Ontology Subrepo and Schema Ingestion | In Review | `ontology-agent + rust-agent` | See `conductor/tracks/57-*/spec.md` |
| 58 | Ontology-to-Rust Component Code Generation | In Review | `ontology-agent + ecs-agent + api-governance-agent` | See `conductor/tracks/58-*/spec.md` |
| 59 | Feature-Gated Graph Relations ECS Module | In Review | `ecs-agent + core-scheduler-agent + conformance-agent` | See `conductor/tracks/59-*/spec.md` |
| 60 | Normal-Form Multi-Game Runtime and Solvers | In Review | `game-theory-agent + ecs-agent + performance-agent` | See `conductor/tracks/60-*/spec.md` |
| 61 | Extensive-Form Graph-ECS Runtime and Certification | In Review | `game-theory-agent + ecs-agent + benchmark-agent + redteam-agent` | See `conductor/tracks/61-*/spec.md` |

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
