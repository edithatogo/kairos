# KairoECS Tracks Index

This file is the human-readable Conductor status index. Keep it in lockstep with `conductor/tracks.yaml`, which is the authoritative machine-readable source for status, ownership, dependencies, paths, and gates.

| Track | Name | Status | Primary subagent | Artifact |
|---:|---|---|---|---|
| 00 | Project Foundation, Governance & Naming | Spec Approved | `foundation-agent` | See `conductor/tracks/00-*/spec.md` |
| 01 | The Heart: kairo-ecs-core & kairo-ecs-state | In Progress | `core-scheduler-agent + ecs-agent + contracts-agent` | See `conductor/tracks/01-*/spec.md` |
| 02 | The Bridge: kairo-ecs-ffi, UniFFI & Diplomat | In Progress | `ffi-agent + uniffi-agent + diplomat-agent` | See `conductor/tracks/02-*/spec.md` |
| 03 | The Flow: DES Trajectory API & ABM Behavior API | In Progress | `des-api-agent + abm-api-agent` | See `conductor/tracks/03-*/spec.md` |
| 04 | The Analyst: kairo-ecs-arrow | In Progress | `arrow-agent` | See `conductor/tracks/04-*/spec.md` |
| 05 | The Window: kairo-ecs-viz | In Progress | `viz-agent` | See `conductor/tracks/05-*/spec.md` |
| 06 | Python Binding 3.10-3.14 | In Progress | `python-agent` | See `conductor/tracks/06-*/spec.md` |
| 07 | R Binding | In Progress | `r-agent` | See `conductor/tracks/07-*/spec.md` |
| 08 | Julia Binding | In Progress | `julia-agent` | See `conductor/tracks/08-*/spec.md` |
| 09 | TypeScript/Wasm Binding | In Progress | `typescript-agent` | See `conductor/tracks/09-*/spec.md` |
| 10 | C# Binding .NET 10-11 | In Progress | `csharp-agent` | See `conductor/tracks/10-*/spec.md` |
| 11 | Go Binding | In Progress | `go-agent` | See `conductor/tracks/11-*/spec.md` |
| 12 | Conformance, Testing & Benchmarks | In Progress | `conformance-agent + performance-agent` | See `conductor/tracks/12-*/spec.md` |
| 13 | CI/CD, Code Quality & Supply Chain | In Progress | `ci-agent + security-agent` | See `conductor/tracks/13-*/spec.md` |
| 14 | Documentation Site & Education | In Progress | `docs-agent` | See `conductor/tracks/14-*/spec.md` |
| 15 | Packaging, Publishing & Delivery | In Progress | `release-agent` | See `conductor/tracks/15-*/spec.md` |
| 16 | Release Governance & Maintenance | In Progress | `release-agent + governance-agent` | See `conductor/tracks/16-*/spec.md` |
| 17 | Community Adoption, Education & Ecosystem | In Progress | `community-agent` | See `conductor/tracks/17-*/spec.md` |
| 18 | Comparative Benchmarks & Reproducibility | In Progress | `benchmark-agent` | See `conductor/tracks/18-*/spec.md` |
| 19 | Research Software, Citation & Archival | In Progress | `research-agent` | See `conductor/tracks/19-*/spec.md` |
| 20 | OpenSSF, Supply Chain Trust & Institutional Readiness | In Progress | `security-agent` | See `conductor/tracks/20-*/spec.md` |
| 21 | Verification, Validation & Uncertainty | In Progress | `vv-uq-agent` | See `conductor/tracks/21-*/spec.md` |
| 22 | Experiment Runner & Scenario Management | In Progress | `experiment-agent` | See `conductor/tracks/22-*/spec.md` |
| 23 | Domain Starter Kits & Model Zoo | In Progress | `model-zoo-agent` | See `conductor/tracks/23-*/spec.md` |
| 24 | Playground, Demos & Visualization UX | In Progress | `playground-agent` | See `conductor/tracks/24-*/spec.md` |
| 25 | API Design Review & Compatibility Governance | In Progress | `api-governance-agent` | See `conductor/tracks/25-*/spec.md` |
| 26 | Interoperability Standards Review | In Progress | `interop-agent` | See `conductor/tracks/26-*/spec.md` |
| 27 | Developer Experience & Reproducible Environments | In Progress | `dx-agent` | See `conductor/tracks/27-*/spec.md` |
| 28 | Red Team & Devil's Advocate Review | In Progress | `redteam-agent` | See `conductor/tracks/28-*/spec.md` |
| 29 | Wave Manager & Execution Gatekeeper | In Progress | `wave-manager-agent` | See `conductor/tracks/29-*/spec.md` |
| 30 | Toolchain & Version Support Matrix | In Progress | `toolchain-agent` | See `conductor/tracks/30-*/spec.md` |
| 31 | Performance Regression Guard | In Progress | `perf-regression-agent` | See `conductor/tracks/31-*/spec.md` |
| 32 | GPU Compute Acceleration | In Progress | `gpu-compute-agent` | See `conductor/tracks/32-*/spec.md` |
| 33 | WebGPU Compute for Browser | In Progress | `webgpu-agent` | See `conductor/tracks/33-*/spec.md` |
| 34 | PDES & Parallel Execution | In Progress | `pdes-agent` | See `conductor/tracks/34-*/spec.md` |
| 35 | Distributed Simulation (MPI/gRPC) | In Progress | `distributed-agent` | See `conductor/tracks/35-*/spec.md` |
| 36 | Streaming & Real-Time Processing | In Progress | `streaming-agent` | See `conductor/tracks/36-*/spec.md` |
| 37 | ML/AI Integration & Inference | In Progress | `ml-integration-agent` | See `conductor/tracks/37-*/spec.md` |
| 38 | FMI/FMU & Digital Twin Bridge | In Progress | `fmi-agent` | See `conductor/tracks/38-*/spec.md` |
| 39 | Cloud/HPC Batch Runners | In Progress | `cloud-agent` | See `conductor/tracks/39-*/spec.md` |
| 40 | Time-Travel Debugging & Interactive Stepping | In Progress | `timetravel-agent` | See `conductor/tracks/40-*/spec.md` |

## Track status vocabulary

```text
Planned -> Spec Approved -> In Progress -> In Review -> Blocked -> Done -> Deferred -> Cancelled
```

Machine-readable track metadata is maintained in `conductor/tracks.yaml`.

## Release criticality

- Tracks 00, 01, 02, 12, 13, 14, 15, 16, 20, 25, 28, 29, and 30 are release-gating for any public release.
- Track 31 is quality-improving but not release-gating.
- Tracks 06 and 09 are recommended v0.1/v0.2 public binding priorities.
- Tracks 07, 08, 10, and 11 can remain preview until C ABI and Arrow schema stability are proven.
- Track 05 must remain optional and non-blocking for headless releases.
- GitHub automation, dependency review, scorecard, SBOM, and release workflows already exist under `.github/`; registry publication manifests remain planned work under Tracks 15 and 20.
- Tracks 34, 35, and 38 are release-critical for distributed simulation, FMI digital-twin, and PDES infrastructure.
- Tracks 32, 33, 36, 37, 39, and 40 are non-release-critical (enhancement only).
