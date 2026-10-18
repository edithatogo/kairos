---
title: KairoECS Documentation
description: Deterministic simulation engine documentation for core runtime, evidence, and polyglot bindings.
---

KairoECS is a Rust-first simulation engine for deterministic event scheduling,
Structure-of-Arrays (SoA) ECS state, discrete-event simulation (DES),
agent-based modeling (ABM), multi-game equilibrium solving, Apache Arrow/Parquet
telemetry, and polyglot bindings across Python, TypeScript/Wasm, R, Julia, C#,
and Go.

This Starlight site is the active documentation shell for the repository. It
provides architectural maps, quickstarts, and capability matrices while linking
directly to the repository-level source documentation and Conductor evidence ledgers.

## Start here

- [Architecture](architecture.md) — 5-tier system diagram, complete 23-crate inventory, and data-flow lifecycle.
- [Docs platform](docs-platform.md) — Astro/Starlight plugin stack, versioning, and `llms.txt` generation.
- [Conductor status](evidence/conductor-status.md) — Track completion matrix (`60/67` tracks `Done`).
- [Rust quickstart](polyglot/rust.md) — Native Rust scheduler, SoA ECS, DES/ABM, and Game Theory usage.
- [Python quickstart](polyglot/python.md) — Python C ABI, zero-copy Arrow export, and `kairo_gym` RL `VecEnv`.
- [PDES and distributed evidence](evidence/pdes-distributed.md) — Conservative & Time Warp PDES, MPI, and gRPC.

## Core Capability Matrix

| Subsystem | Primary Crates | Highlights |
| :--- | :--- | :--- |
| **Deterministic Core & SoA ECS** | `kairo-ecs-core`, `kairo-ecs-state`, `kairo-ecs-rng`, `kairo-ecs-types` | 64-bit `SimTime`, `(timestamp, priority, sequence)` event ordering, dense `ArchetypeTable` columns, `EventArena`, SplitMix64 substreams |
| **DES, ABM & Game Theory** | `kairo-ecs-des`, `kairo-ecs-abm`, `kairo-ecs-game-ontology`, `kairo-ecs-game-theory` | SimPy-style `SimResource`/`PreemptiveResource`/`Container`/`Store`, `GridSpace2D`/`ContinuousSpace2D`/`NetworkSpace`, Pure Nash, Lemke-Howson, Backward Induction & CFR+ |
| **Parallel, Distributed & GPU** | `kairo-ecs-pdes`, `kairo-ecs-mpi`, `kairo-ecs-grpc`, `kairo-ecs-gpu`, `kairo-ecs-webgpu` | Chandy-Misra-Bryant lookahead, Jefferson Time Warp rollback + Mattern GVT, Apple Silicon `MetalBackend` unified memory, WGSL browser compute |
| **Data Spine, ML/RL & Digital Twin** | `kairo-ecs-arrow`, `kairo-ecs-ml`, `kairo-ecs-fmi`, `kairo-ecs-cli`, `kairo-ecs-debug` | Apache Arrow 53 `RecordBatch`, Parquet Zstd, C Data Interface FFI, multi-core `ParallelVecEnv`, Sobol/LHS/ABC-SMC ensemble runner, CoW time-travel snapshots |
| **Polyglot Language Bridges** | `kairo-ecs-ffi`, `kairo-ecs-wasm`, `kairo-ecs-uniffi`, `kairo-ecs-diplomat` | Stable `include/kairo_ecs.h` C ABI, WebAssembly `WasmSimulation` & `WasmTimeTravelInspector`, Python/R/Julia/TS/C#/Go bindings |

## Quickstart Example (Rust)

```rust
use kairo_ecs_core::Scheduler;
use kairo_ecs_types::{EventId, Priority, SimTime};
use kairo_ecs_des::SimResource;
use kairo_ecs_abm::GridSpace2D;

let mut scheduler = Scheduler::new();
scheduler.schedule(SimTime::from_millis(10), Priority::NORMAL, EventId::new(1));

let mut server = SimResource::new(2);
assert!(server.request(101, 1, 0));

let mut grid = GridSpace2D::new(32, 32, true);
grid.place(101, (4, 7));
assert_eq!(grid.moore_neighbors((4, 7)).len(), 0);
```

## Source documentation

The full source documentation remains in the repository-level `docs/`,
`bindings/`, `examples/`, and `conductor/` trees. The Starlight shell links back
to those canonical files where a page is still maintained outside the website
content collection.

## Local workflow

```powershell
npm --prefix website run check:all
npm --prefix website run build
npm --prefix website run dev
just docs-build
just docs-dev
```
