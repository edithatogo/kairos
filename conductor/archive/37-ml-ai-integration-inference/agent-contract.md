# Agent Contract: ml-integration-agent

## Track

Track 37: ML/AI Integration & Inference

## Owned paths

- `crates/kairo-ecs-ml/`
- `docs/ml/`
- `examples/ml-surrogate/`
- `python/kairo_gym/`
- `conductor/tracks/37-ml-ai-integration-inference/`

## Required handoff

- Summary of artifacts produced (crate, Python package, examples, docs).
- Feature flag matrix: which flags enable which backends and their dependencies.
- ONNX Runtime and TensorRT version compatibility matrix.
- Gymnasium environment contract: observation space, action space, reward function specification.
- Neural surrogate accuracy benchmark results for the test domain.
- Risks discovered and unresolved questions.

## Prohibited changes without ADR

- Public Rust APIs in `kairo-ecs-core` or `kairo-ecs-runtime`.
- C ABI signatures.
- Arrow schema field semantics.
- ECS tick lifecycle or scheduler behavior.
- GPU memory allocation policy owned by Track 32.
- Python binding surface owned by Track 06.

## Feature flag governance

- `onnx`: gates ONNX Runtime dependency; must be off by default.
- `tensorrt`: gates TensorRT dependency; requires NVIDIA GPU and CUDA toolkit.
- `burn`: gates Burn framework dependency; no external shared libraries required.
- `gymnasium`: gates Python Gymnasium wrapper build; requires PyO3 and Python >= 3.10.

## Integration points

- Consumes ECS `World::tick_begin` and `World::tick_end` hooks from Track 01.
- Consumes GPU device handle and memory budget from Track 32.
- Consumes PyO3 module registration from Track 06.
- Provides trained model inference results to any ECS system that implements `NeuralReceive`.
