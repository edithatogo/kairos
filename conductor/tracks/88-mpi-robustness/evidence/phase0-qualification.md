# Phase 0 Evidence & Value Gate Qualification: rsmpi Native MPI Qualification

**Track:** 88-mpi-robustness  
**Date:** 2026-10-25  
**Lead:** `distributed-agent`  
**Status:** Phase 0 Complete — Evaluation Decision: **REJECT (NO-GO)**

---

## 1. Inventory & Target Scope (Task 0.1)

- **Lead Agent:** `distributed-agent`
- **Owned Path:** `conductor/tracks/88-mpi-robustness/`
- **Prospective Target:**
  Candidate replacement of distributed discrete-event and PDES message transport in `crates/kairo-ecs-mpi/` with native Message Passing Interface (`rsmpi` / `mpi = "0.8.2"`).
- **Prerequisites Status:**
  - Track 78 (`Rayon data parallelism robustness`) completed and qualified.
  - Track 81 (`Prost distributed envelope encoding`) completed and qualified.
  - Track 82 (`Tokio bounded transport runtime`) completed and qualified.
  - Track 83 (`Tonic interoperable RPC transport`) completed and qualified.
- **Current Baseline Capabilities in Kairos Distributed Transports:**
  1. **Pure Rust Portability & Safety:**
     - `crates/kairo-ecs-mpi` is pure Rust and strictly enforces `#![forbid(unsafe_code)]`.
     - `crates/kairo-ecs-grpc` (`TonicTransport`) and pure-Rust TCP socket transports require zero external C compiler, zero external runtime services, and zero host C/C++ library dependencies.
  2. **Interoperable PDES Protocol:**
     - High-performance Jefferson Time Warp optimistic envelopes with deep ancestry trees, state rollbacks, and anti-messages.
     - Dynamic entity migration with generational ID preservation (`EntityId`).
     - Asynchronous Mattern GVT consensus rounds with deterministic ordering.
     - Protobuf v2 schema compliance with dual-compatibility for legacy envelopes.
  3. **Zero Host Environment Friction:**
     - Fully builds, tests, and verifies out-of-the-box on standard GitHub Actions runners (`ubuntu-latest`, `macos-15`, `windows-latest`) with exact Rust 1.99.1.

---

## 2. Dependency Qualification (Task 0.2)

- **Candidate Crate:** `mpi` (rsmpi bindings on crates.io)
- **Version:** `0.8.2`
- **Repository:** `https://github.com/rsmpi/rsmpi`
- **License:** Apache-2.0 OR MIT
- **Internal Architecture:**
  - `mpi` is a Rust wrapper around the C Message Passing Interface (MPI) standard.
  - Uses `mpi-sys` and `build.rs` probing `mpicc` / `pkg-config` / `MPI_LIB` environment variables to link host dynamic or static C MPI runtimes (OpenMPI, MPICH, MS-MPI).
- **Critical Impedance Mismatches & Robustness Risks:**
  1. **Heavy External Toolchain & C ABI Requirement:**
     - Requires a native MPI implementation pre-installed on the host operating system (`openmpi-bin`, `libopenmpi-dev`, or `mpich`).
     - Fails to compile on any standard machine without a configured C MPI toolchain.
     - Severely complicates multi-platform CI, developer local bootstrapping, and containerless deployments.
  2. **Substantial Unsafe Code & Undefined Behavior Surface:**
     - MPI C FFI functions operate via untyped raw `void*` pointers, manual rank indices, and non-blocking request handles.
     - Misaligned buffers, rank mismatches, or concurrent request buffer reuse invoke hard process segfaults that bypass Rust's panic infrastructure.
     - Violates the `#![forbid(unsafe_code)]` contract maintained in `crates/kairo-ecs-mpi`.
  3. **Concurrency & Event Loop Conflicts:**
     - MPI thread safety modes (`MPI_THREAD_SINGLE`, `MPI_THREAD_FUNNELED`, `MPI_THREAD_SERIALIZED`, `MPI_THREAD_MULTIPLE`) vary widely across platform builds.
     - Bridging native MPI polling loops into Tokio's async reactor (`TonicTransport`) or Rayon work-stealing pools causes thread contention, priority inversions, and unpredictable deadlock risks.

---

## 3. Value Gate Evaluation (Task 0.3)

Pursuant to the mandatory rules in [dependency-value-policy.md](../../dependency-value-policy.md) and the track contract in [spec.md](../spec.md):

1. **Stop Criterion Verification:**
   > *"Stop criterion: No installation for socket-only tests or absent HPC consumer; no CPU/socket fixture promoted to MPI acceptance."*
   - Neither `kairos` nor `CareOps-Sim` currently has an active HPC consumer or supercomputer cluster deployment requiring native InfiniBand/Omni-Path MPI collectives.
   - All multi-process, distributed agent, and multi-node PDES requirements are completely satisfied by the pure-Rust `TonicTransport` (HTTP/2 gRPC via `tonic` + `prost`, Track 83) and Tokio socket transports (Track 82).
   - Under the explicit Stop Criterion of Track 88, installing native MPI in the absence of an active HPC consumer is strictly prohibited.
2. **Value Policy Rule on Conditional Evaluations:**
   > *"If there is no production target/accepted consumer, no net benefit, unsupported contracts or excessive migration burden, record no-go/deferred with evidence. Keep Phase 1/2 unchecked, adoption_decision=deferred/rejected, and do not install a dependency merely to close a track. Close/archive a rejected evaluation only with an explicit outcome; never label it implemented."*
   - Adopting `mpi = "0.8.2"` would introduce negative net value: it introduces external C dependencies, weakens memory safety guarantees, breaks standard runner CI, and provides zero incremental benefit over the verified pure-Rust gRPC transport.

---

## 4. Evaluation Outcome & Disposition (Task 0.4)

- **Phase 0 Value Gate Decision:** **REJECT (NO-GO)**
- **Adoption Decision:** `rejected`
- **Disposition:**
  - Do NOT install `mpi` / `rsmpi`.
  - Retain the pure-Rust `#![forbid(unsafe_code)]` `kairo-ecs-mpi` contract emulator and production `kairo-ecs-grpc` (`TonicTransport`) / `kairo-ecs-tokio` transports.
  - Leave Phase 1 (Bounded replacement) and Phase 2 (Acceptance & handoff) unselected, keeping this track cleanly closed at Phase 0 evaluation per policy.
