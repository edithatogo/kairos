---
title: Architecture
description: KairoECS runtime and documentation architecture map.
---

KairoECS is organized around a small deterministic core and optional advanced
execution surfaces.

## Runtime layers

- Core contracts: `kairo-ecs-types`, `kairo-ecs-core`, `kairo-ecs-state`, and
  `kairo-ecs-rng`.
- Simulation modes: DES and ABM crates with deterministic fixture coverage.
- Telemetry: Arrow schema documentation and versioned event-log contracts.
- Advanced execution: GPU, WebGPU, PDES, distributed MPI/gRPC, cloud/HPC, ML,
  streaming, FMI, and time-travel debugging tracks.

## Documentation layers

- Starlight shell under `website/`.
- Canonical source docs under repository-level `docs/`.
- Conductor evidence under `conductor/`.
- Polyglot binding guides under `bindings/` and Starlight polyglot entry pages.

## Release posture

The project is still pre-release. Stable API, package publication, and external
runtime claims remain gated by the conductor compatibility and release tracks.
