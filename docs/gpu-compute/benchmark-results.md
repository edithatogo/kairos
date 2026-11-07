# GPU execution and benchmark evidence

Updated 7 November 2026. GPU implementation and bounded device qualification
exist; a general model-acceleration or performance-superiority claim is not
established.

## What is implemented and tested

The [native GPU model contract](../../crates/kairo-ecs-gpu/src/compute.rs)
contains particle positions/velocities and integer entity values. The
[Metal qualification tests](../../crates/kairo-ecs-gpu/tests/metal_hardware_qualification.rs)
exercise device identification, persistent buffers over ten ticks for 10,000
particles, and a CPU comparison with 2,048 particles. Retained
[execution evidence](../../conductor/tracks/107-careops-staged-flow-compatibility/evidence/just-ci-mainline.log)
records hardware-test passes for its historical source/toolchain.

The floating-point comparison accepts absolute error up to `5e-5`, despite its
historical test name containing `bit_for_bit`. That establishes tolerance-based
agreement for the tested operation, not exact equality across devices or arbitrary
DES/ABM models. The DES shader's bounded integer updates are not a general GPU
event scheduler.

The older [CPU parity scaffold](../../conductor/hpc-evidence/manifests/track52-local-cpu-parity-scaffold.json)
remains historical contract evidence. It cannot stand in for hardware execution,
and a successful device kernel cannot stand in for an end-to-end model benchmark.

## Required performance comparison

Before claiming GPU speedup, record hardware and driver identity, backend,
source/feature/build profile, model definition, agent/event counts, random-stream
policy, warm-up, raw repeated timings and memory consumption. Include compilation
and upload/download costs according to explicitly stated timing boundaries.
Compare with an optimized Rust CPU implementation of the same model and report
the workload sizes at which GPU execution helps or loses.

Use [Track 52's hardware manifest](../../conductor/hpc-evidence/manifests/track52-live-gpu-hardware-template.json)
for the applicable source-bound qualification; use the
[measured-performance programme](../../conductor/tracks/111-measured-performance/spec.md)
for complete-model comparisons. Distinguish exact integer/discrete replay,
floating-point tolerance and statistical equivalence in every result.
