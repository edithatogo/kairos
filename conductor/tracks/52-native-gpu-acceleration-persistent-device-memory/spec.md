# Track 52: Native GPU Acceleration with Persistent Device Memory

## Purpose

Replace GPU placeholders with real `wgpu` and CUDA-backed execution, persistent
device memory, kernel/batch dispatch, CPU parity tests, and hardware benchmark
evidence.

## Maturity

Spec Approved planning track. The current implementation remains the Track 32
and Track 33 fallback/placeholder surfaces until real hardware proof closes.

## Inputs

- `crates/kairo-ecs-gpu/`, `crates/kairo-ecs-webgpu/`, and docs from Tracks 32
  and 33.
- NUMA and memory layout constraints from Track 50.
- Evidence manifest from Track 46.

## Outputs

- Real `wgpu` backend behind `wgpu-backend`.
- Real CUDA backend behind `cuda-backend`.
- Persistent device buffers across simulation ticks.
- CPU/GPU parity tests for DES, ABM, and mixed transition batches.
- Hardware benchmark evidence with driver and device metadata.

## Owned paths

- `crates/kairo-ecs-gpu/`
- `crates/kairo-ecs-webgpu/`
- `docs/gpu-compute/`
- `website/webgpu-demo/`
- `conductor/tracks/52-native-gpu-acceleration-persistent-device-memory/`

## Blocked paths

- Core scheduler semantics without Track 01 handoff.
- NUMA allocator internals owned by Track 50.
- End-to-end scaling certification owned by Track 55.

## Dependencies

Tracks 32, 33, 46, and 50.

## Parallel-safe tracks

Track 54 may draft GPU job templates after this track defines device runtime
requirements. Track 55 may consume benchmark evidence after hardware tests run.

## Acceptance criteria

- Backend initialization uses real device APIs and reports typed errors when no
  device is available.
- Device buffers persist across ticks and avoid unnecessary host copies.
- GPU batch results match CPU reference results across deterministic scenarios.
- CUDA and wgpu hardware evidence records device, driver, workload, and raw
  timings.
- Public docs do not claim speedup without hardware evidence.

## Quality gates

- `real-wgpu-device-dispatch`
- `real-cuda-kernel-dispatch`
- `persistent-device-memory`
- `gpu-cpu-parity`
- `gpu-benchmark-threshold`
- `phase-closeout-check`

## Release implications

This track gates production GPU acceleration claims. CPU fallback behavior must
remain correct on hosts without GPU support.
