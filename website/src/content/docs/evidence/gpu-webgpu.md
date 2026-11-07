---
title: GPU and WebGPU
description: Device kernel and persistent-buffer evidence, with explicit numerical and performance boundaries.
---

The Rust GPU and WebGPU crates contain device kernels and persistent-buffer
paths. Support must be described per backend, device, kernel and execution
receipt. A common trait or a CPU reference does not qualify every backend.

## Bounded hardware evidence

The [Metal qualification tests](https://github.com/edithatogo/kairos/blob/main/crates/kairo-ecs-gpu/tests/metal_hardware_qualification.rs)
exercise device identification, 10,000 particles over ten persistent-buffer
ticks and a CPU comparison using 2,048 particles. Retained historical
[execution output](https://github.com/edithatogo/kairos/blob/main/conductor/tracks/107-careops-staged-flow-compatibility/evidence/just-ci-mainline.log)
records hardware-test passes for its source and toolchain.

The floating-point comparison accepts absolute error up to `5e-5`. A test name
containing `bit_for_bit` does not change that tolerance into exact equality.
The particle update and bounded integer DES shader do not establish arbitrary
model offload or a complete GPU event scheduler.

## Claims that need their own measurements

| Claim | Required evidence |
| --- | --- |
| Actual GPU execution | Device/driver identity, backend, enabled features, successful kernel dispatch and source-bound logs. |
| Numerical agreement | Model-specific exact, tolerance or statistical rule; independent reference and edge cases. |
| End-to-end acceleration | Identical model/workload, optimized Rust CPU baseline, repeated timings and explicit compilation/transfer boundaries. |
| Persistent or zero-copy data | Verified ownership/lifetime and measured movement at the particular boundary; no extrapolation to the whole workflow. |
| Broad backend support | Separate installed hardware/browser acceptance for each advertised backend and platform. |

Read [GPU benchmark evidence](https://github.com/edithatogo/kairos/blob/main/docs/gpu-compute/benchmark-results.md),
[free testing routes](https://github.com/edithatogo/kairos/blob/main/docs/gpu-compute/free-testing-routes.md)
and the [capability register](https://github.com/edithatogo/kairos/blob/main/conductor/capabilities.json).
General performance leadership and universal cross-device bitwise parity have
not been established by the retained kernel tests.
