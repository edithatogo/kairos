# ML inference architecture

`Tensor` validates shape, checked element count and finite values. `ModelMetadata` records the model name, version and tensor shapes. `InferenceTickHook` filters by phase and validates inputs and outputs around a registered `NeuralSystem`.

`AffineSurrogate` implements an affine transformation in Rust. It accepts a finite coefficient matrix and bias, validates each flattened input against its calibration range, and checks the resulting tensor.

| Runtime | Current status |
| --- | --- |
| Rust affine surrogate | Runs from caller-supplied coefficients and declared bounds |
| ONNX Runtime | The descriptor validates metadata and shape. Execution returns an unavailable error |
| TensorRT and Burn | Unavailable |
| Rust queue and host adapters | Experimental queue model, batched Rust execution and opt-in Gymnasium and PettingZoo adapters |

Construction of an ONNX descriptor and compilation with the `onnx` feature provide shape and metadata checks. The [surrogate guide](surrogate-authoring.md) describes executable inference and its held-out test.
