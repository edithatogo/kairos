# ML Inference Architecture

Track 37 starts with optional inference surfaces that compile without runtime ML
dependencies. The crate boundary is:

- `Tensor`: shape-checked inference payload.
- `OrtSession`: ONNX Runtime-facing session wrapper surface. The current scaffold
  performs deterministic shape-checked passthrough inference until the real ONNX
  dependency is introduced.
- `NeuralSystem`: simulation tick hook contract for model-backed systems.
- `InferenceTickHook`: phase-filtered runner that can execute models before or
  after the ECS systems owned by Track 01.

All backends remain feature-gated:

| Feature | Backend | Current state |
|---|---|---|
| `onnx` | ONNX Runtime wrapper | scaffold alias |
| `tensorrt` | NVIDIA TensorRT | blocked on Track 32 GPU memory contract |
| `burn` | Pure Rust inference | scaffold alias |
| `gymnasium` | Python/RL integration | Rust-side space contract |

The default build must not link ONNX Runtime, TensorRT, Burn, or Python.
