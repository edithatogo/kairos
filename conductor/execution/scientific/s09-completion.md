# S09 Native inference

Keep the optional ONNX boundary unavailable until a qualified ONNX runtime and model exist. Feature compilation alone cannot make that backend available. Reuse the existing `NeuralSystem` and tensor validation for a Rust-native affine surrogate over a declared input domain. Include model identity and version with caller-supplied coefficients. Reject incompatible shapes, non-finite coefficients, overflowing output and out-of-domain inputs.

Check a non-identity matrix against hand-calculated values. Use independently computed exponential decay over the bounded training domain as a held-out surrogate test with a maximum absolute error of 0.025. Preserve the scheduler hook and its fallback observability.
