# Track 37: ML/AI Integration & Inference

## Purpose

Embed ML inference directly in the simulation loop. Support ONNX Runtime for cross-platform inference, TensorRT for NVIDIA GPU-optimized inference, and neural surrogate models that replace slow physics/behavior sub-models for 100-1000x speedup. Provide a Gymnasium-compatible Python environment for RL training.

## Why this track exists

Simulation models increasingly rely on learned components: surrogate models that approximate expensive PDE solvers, RL-trained policies for agent behavior, and neural networks that replace heuristic sub-models. Without first-class ML inference support, users must hand-roll bespoke integrations or accept orders-of-magnitude slower simulations. This track brings ONNX Runtime, TensorRT, and Burn (Rust-native ML) into the KairoECS loop with safe, non-blocking inference.

## Primary subagent

`ml-integration-agent`

## Dependencies

- Track 23: Domain Starter Kits & Model Zoo — provides example domains that benefit from surrogate models.
- Track 06: Python Binding — required for Gymnasium environment wrapper and RL training integration.
- Track 32: GPU Compute Acceleration — coordinates GPU memory and compute resources with ML inference.
- Track 01: Heart/KairoECS Core — consumes ECS tick boundaries for inference scheduling.

## Owned paths

```text
crates/kairo-ecs-ml/, docs/ml/, examples/ml-surrogate/
```

## Blocked paths

```text
crates/kairo-ecs-core/ — owned by Track 01 (core scheduler)
crates/kairo-ecs-runtime/ — owned by Track 29 (wave manager, execution gating)
```

## Inputs

- Domain models from Track 23 (example domains for surrogate validation).
- Python binding from Track 06 (Gymnasium environment wrapper).
- ONNX Runtime v1.18+ C API bindings via `ort` or `tract` crate.
- TensorRT 10.x Rust bindings via `tensorrt-rs` or generated C bindings.
- Burn (Rust-native ML framework) for pure-Rust inference paths.
- GPU compute contracts from Track 32 for memory and stream coordination.

## Outputs

- `crates/kairo-ecs-ml/`: crate with ONNX model loading, session management, and inference at simulation tick boundaries.
- Neural surrogate API: trait `NeuralSystem` that wraps any ECS system and replaces its logic with a trained model.
- Gymnasium environment wrapper: Python package `kairo_gym` exposing KairoECS as a `gymnasium.Env`.
- `docs/ml/`: architecture overview, model versioning policy, and surrogate authoring guide.
- `examples/ml-surrogate/`: end-to-end example replacing a DE system with an ONNX surrogate.

## Acceptance criteria

- ONNX model loads, runs inference inside a simulation step, and does not deadlock under concurrent tick execution.
- Gymnasium environment returns `obs`, `reward`, `terminated`, `truncated`, and `info` according to the Gymnasium v0.29+ specification.
- Neural surrogate produces state output within 5% relative error of the original system for the test domain under 1000 consecutive ticks.
- All ML dependencies are optional behind Cargo feature flags (`onnx`, `tensorrt`, `burn`, `gymnasium`).

## Non-goals

- Training models inside KairoECS (training is external; only inference is in-loop).
- Replacing the core scheduler with an ML-driven scheduler.
- General-purpose MLOps pipeline or model registry.
- Pre-packaged trained models (users provide their own).

## Release implications

- Non-blocking; all ML functionality is gated behind feature flags.
- ONNX Runtime shared library is an optional runtime dependency, not bundled.
- Default `kairo-ecs` crate compiles and passes all tests without ML features enabled.
- Gymnasium environment wrapper is distributed as a separate Python package (`kairo-gym`), not via crates.io.

## Status

Planned
