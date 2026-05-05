# Track 18: Comparative Benchmarks & Reproducibility

## Purpose

Publish reproducible, fair benchmarks against established DES/ABM ecosystems and measure binding overhead.

## Why this track exists

KairoECS is not only a Rust kernel. It is a multi-language research and engineering ecosystem. This track protects the project from the most common failure mode for ambitious open-source infrastructure: impressive internals with insufficient trust, examples, packaging, governance, and contributor experience.

## Primary subagent

`benchmark-agent`

## Parallelization model

This track is designed to run in parallel with core implementation. The subagent owns docs, policies, examples, checklists, manifests, fixtures, and automation controls. It must not block kernel development unless it identifies a release-blocking risk.

## Inputs

- `conductor/contracts/core-contract.md` (scheduler API surface under test).
- `conductor/contracts/ffi-contract.md` (binding overhead targets for FFI benchmarks).
- `conductor/contracts/conformance-contract.md` (scenario definitions shared with Track 21).
- Published DES/ABM benchmark baselines (PHOLD, DEVStone) and their canonical model configs.
- Handoff notes from Tracks 02 (FFI) and 21 (VVUQ scenario fixtures).

## Outputs

- `benchmarks/`: Rust benchmark harness under `crates/kairo-ecs-core/benches/` with criterion-based measurement.
- `benchmarks/baselines/`: reference results from comparable ecosystems (ADEVS, PowerDEVS, SimPy) with metadata.
- `benchmarks/methodology.md`: benchmark design, hardware requirements, result interpretation guide, and caveats.
- `benchmarks/scenarios/`: scenario definition files (JSON/YAML) with seed, entity count, step count, and measurement config.
- `conductor/tracks/18-comparative-benchmarks-reproducibility/test-matrix.md`: CI gate (benchmark runs without regression, metadata completeness check).

### Extended comparator framework list

In addition to SimPy, simmer, Mesa, Agents.jl, MASON, and NetLogo, the benchmark suite SHALL include comparisons against:

- **FLAME GPU** — GPU-accelerated ABM (agent-based model) for GPU parity benchmarking (Track 32).
- **Repast HPC** — distributed ABM for multi-node parity benchmarking (Track 35).
- **NS-3** — network simulation for DES event dispatch throughput comparison.
- **OMNeT++** — discrete event network simulation for scheduler performance comparison.
- **SimGrid** — distributed systems simulation for MPI/gRPC parity (Track 35).
- **µsik / WarpIV / ROSS** — PDES engines for parallel execution parity (Track 34).
- **EcoLab / Golly** — cellular automata for RNG/state update throughput comparison.

## Acceptance criteria

- The track has a clear public-facing output, not just internal notes.
- The output is testable, reviewable, or linked to a release gate.
- It includes failure modes and countermeasures.
- It does not duplicate core implementation work owned by Tracks 01-05.
- It supports at least one of: adoption, trust, reproducibility, maintainability, or compatibility.

## Non-goals

- Replacing the core scheduler or ECS design.
- Publishing packages before naming, legal, security, and compatibility gates pass.
- Adding domain-specific complexity to `kairo-ecs-core`.



