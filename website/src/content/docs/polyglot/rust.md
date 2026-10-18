---
title: Rust
description: Native Rust workspace architecture, crate APIs, and deterministic simulation examples.
---

Rust is the primary implementation language of KairoECS. All 23 workspace crates enforce `#![forbid(unsafe_code)]` outside explicit FFI boundary modules (`kairo-ecs-ffi`, `kairo-ecs-arrow::ffi`, `kairo-ecs-fmi`).

## Key Rust Crates by Domain

| Domain | Crates | Primary Types |
| --- | --- | --- |
| Core ECS & Scheduling | `kairo-ecs-types`, `kairo-ecs-core`, `kairo-ecs-state`, `kairo-ecs-rng` | `SimTime`, `EntityId`, `Scheduler`, `ArchetypeTable`, `DeterministicRng` |
| Discrete-Event & ABM | `kairo-ecs-des`, `kairo-ecs-abm` | `SimResource`, `PriorityResource`, `PreemptiveResource`, `Container`, `Store`, `GridSpace2D`, `ContinuousSpace2D`, `NetworkSpace` |
| Game Theory & Ontology | `kairo-ecs-game-ontology`, `kairo-ecs-game-theory` | `NormalFormGame`, `PureNashSolver`, `LemkeHowsonSolver`, `SupportEnumerationSolver`, `ExtensiveFormGame`, `BackwardInductionSolver`, `CfrPlusSolver` |
| Parallel, Distributed & GPU | `kairo-ecs-pdes`, `kairo-ecs-mpi`, `kairo-ecs-grpc`, `kairo-ecs-gpu`, `kairo-ecs-webgpu` | `TimeWarpPartition`, `MatternGvtTracker`, `MpiProcessTransport`, `GrpcSocketTransport`, `MetalBackend`, `PersistentGpuSession` |
| Data, ML, FMI & CLI | `kairo-ecs-arrow`, `kairo-ecs-ml`, `kairo-ecs-fmi`, `kairo-ecs-debug`, `kairo-ecs-cli` | `ArrowRecordBatch`, `ParquetZstdCheckpointWriter`, `ParallelVecEnv`, `FmuPackage`, `CowWorldSnapshot`, `ParallelEnsembleRunner` |

## Example: Game Theory + Multi-Core Ensemble Execution

```rust
use kairo_ecs_cli::ensemble::ParallelEnsembleRunner;
use kairo_ecs_game_theory::{LemkeHowsonSolver, NormalFormGame};

// Solve a 2x2 normal-form game via Lemke-Howson linear complementarity
let matching_pennies = NormalFormGame::new_2p(
    2,
    2,
    vec![1.0, -1.0, -1.0, 1.0],
    vec![-1.0, 1.0, 1.0, -1.0],
).unwrap();
let eq = LemkeHowsonSolver::solve(&matching_pennies).unwrap();
assert!((eq.row_strategy[0] - 0.5).abs() < 1e-6);

// Run a deterministic parameter sweep across OS worker threads
let runner = ParallelEnsembleRunner::with_threads(2026, 4);
let points: Vec<Vec<f64>> = (0..64).map(|i| vec![i as f64 * 0.1]).collect();
let outputs = runner.run_sweep(&points, |seed, params| {
    (seed % 100) as f64 * 0.01 + params[0]
});
assert_eq!(outputs.len(), 64);
```

Canonical source: `crates/README.md`.
