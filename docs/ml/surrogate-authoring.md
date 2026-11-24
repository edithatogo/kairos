# Surrogate authoring guide

Use `AffineSurrogate` for an affine approximation over a known calibration domain. Supply a model identity and version, one matrix row per flattened output, a bias vector, and a finite range for each flattened input. Implement `NeuralSystem` directly when the model requires other Rust operations.

1. Freeze the training inputs and fitted coefficients with the model version.
2. Validate the expected input and output shapes.
3. Compare predictions with the original system on held-out points. Choose an error threshold before comparing.
4. Register the model for the intended tick phase and choose the caller's fallback policy.

The [unit-rate decay example](../../examples/ml-surrogate/de-surrogate/src/main.rs) runs a secant approximation trained at time 0 and 0.4. The independent exponential solution gives a maximum absolute residual of about 0.016443 on 39 held-out interior points. Extrapolation returns an error.

```sh
cargo run --manifest-path examples/ml-surrogate/de-surrogate/Cargo.toml
cargo test --locked -p kairo-ecs-ml --test maturity_native_inference
```

The opt-in ONNX Runtime CPU adapter executes qualified fixed-shape float32 models.
TensorRT and Burn execution remain unavailable. Match accuracy claims to the
model and held-out inputs that were qualified.
[ONNX CPU contract and native results](../adr/0026-onnx-cpu-inference.md)
