# Surrogate Authoring Guide

The initial surrogate example lives in `examples/ml-surrogate/de-surrogate`.
It demonstrates the API shape for replacing a small differential-equation update
with a shape-checked model session.

Authoring steps:

1. Export a model to ONNX outside KairoECS.
2. Record the model name, version, input shape, and output shape.
3. Wrap the session in `OrtNeuralSystem`.
4. Register it with `InferenceTickHook` for `BeforeSystems` or `AfterSystems`.
5. Validate output against the original system and record the relative error.

The alpha scaffold uses deterministic passthrough inference so tests can validate
the hook contract without bundling ONNX Runtime. This is not a surrogate
accuracy benchmark. Real authoring work must record the configured backend,
model version, input shape, output shape, and comparison run before claiming a
surrogate replaces a deterministic system.

At the tick boundary, Track 37 validates:

- model metadata during `try_register`
- duplicate `(model name, model version)` rejection during `try_register`
- input tensor shape before `predict`
- output tensor shape after `predict`
- finite tensor values at tensor construction

## Tutorial: dependency-free surrogate scaffold

Run the scaffold checks before wiring a real backend:

```bash
cargo check --manifest-path crates/kairo-ecs-ml/Cargo.toml --all-features --tests
cargo check --manifest-path examples/ml-surrogate/de-surrogate/Cargo.toml --features onnx
set PYTHONPATH=python\kairo_gym\src
python -m unittest discover -s python\kairo_gym\tests
```

The Rust checks prove the feature-gated API surface and example compile. The
Python check proves the Gymnasium-style reset/step/close envelope imports and
works without installing optional Gymnasium.

## Evidence boundary

This tutorial does not execute ONNX Runtime, TensorRT, Burn, Stable-Baselines3,
or a real surrogate accuracy comparison. A tutorial can claim real surrogate
readiness only after it records the backend, model artifact hash, validation
dataset, baseline system, relative-error threshold, and measured latency.
