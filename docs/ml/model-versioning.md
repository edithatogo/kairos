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

Fallback policies:

- `HoldLastOutput`: use the last successful prediction.
- `UseOriginalSystem`: run the deterministic non-ML system.
- `FailTick`: stop the tick and return an inference error.
