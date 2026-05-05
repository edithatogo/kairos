# Model Versioning Policy

Every model used by `kairo-ecs-ml` must carry:

- model name
- semantic or content-addressed version
- input tensor shape
- output tensor shape
- fallback policy

Runtime code must reject tensor shape mismatches before inference. Model staleness
is treated as a release risk when a surrogate replaces domain logic and no
validation run exists for the current model version.

The tick-hook registry rejects duplicate `(model name, model version)` entries
when `try_register` is used. Registering two systems with the same model identity
would make audit logs ambiguous and can hide stale-model deployments.

Tensor payloads must contain only finite `f32` values. NaN and infinity are
rejected at tensor construction so invalid model inputs do not enter the
simulation tick path.

Until real backends are wired, `BackendStatus::NotConfigured` is the expected
state for ONNX Runtime, TensorRT, and Burn. A scenario must not treat the
dependency-free scaffold as proof that a real model backend is available.

Fallback policies:

- `HoldLastOutput`: use the last successful prediction.
- `UseOriginalSystem`: run the deterministic non-ML system.
- `FailTick`: stop the tick and return an inference error.
