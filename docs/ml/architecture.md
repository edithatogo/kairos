# ML inference architecture

`Tensor` validates shape, checked element count and finite values. `ModelMetadata` records the model name, version and tensor shapes. `InferenceTickHook` filters by phase and validates inputs and outputs around a registered `NeuralSystem`.

`AffineSurrogate` implements an affine transformation in Rust. It accepts a finite coefficient matrix and bias, validates each flattened input against its calibration range, and checks the resulting tensor.

| Runtime | Current status |
| --- | --- |
| Rust affine surrogate | Runs from caller-supplied coefficients and declared bounds |
| ONNX Runtime | Optional native CPU inference for fixed-shape float32 models, qualified on Linux x86-64 and macOS ARM64 |
| TensorRT and Burn | Unavailable |
| Rust queue and host adapters | Experimental queue model, batched Rust execution and opt-in Gymnasium and PettingZoo adapters |

Enable the `onnx` Cargo feature and pass the application's initialized
`onnx::Environment` to `OrtSession::with_cpu_runtime`. A standalone application
can call `onnx::init_from` with the absolute path to a trusted installed library
before other ONNX Runtime calls. Applications already using ONNX Runtime pass
their existing environment.

Each bound model declares one float32 input and one float32 output with fixed
shapes. Use `runtime_status` to check binding on a model. Clones of a bound
session share serialized access to inference.

The example evaluates the fixed-step unit-decay fixture with ONNX Runtime 1.32.0.
Replace the library path with the absolute path on your host.

```bash
cargo run --manifest-path examples/ml-surrogate/de-surrogate/Cargo.toml \
  --features onnx -- /absolute/path/to/libonnxruntime.so.1.32.0 \
  crates/kairo-ecs-ml/tests/data/unit-decay-v1.onnx
```

[Runtime decision](../adr/0026-onnx-cpu-inference.md) · [Affine surrogate guide](surrogate-authoring.md).
