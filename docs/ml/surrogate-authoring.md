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
the hook contract without bundling ONNX Runtime.
