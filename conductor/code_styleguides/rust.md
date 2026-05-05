# Rust Style Guide

- Keep hot-path crates small and dependency-light.
- Use `thiserror` for library errors; avoid `anyhow` in public library APIs.
- Forbid `unsafe` in `kairo-ecs-core`, `kairo-ecs-types`, and `kairo-ecs-state` unless an ADR permits it.
- Isolate unsafe FFI code in bridge crates and document each unsafe block.
- Use fixed simulation time, not `f64`, in scheduler ordering.
- Use typed handles for events, entities, resources, simulations, telemetry batches, and scenarios.
- Add property tests for ordering, cancellation, replay, and handle lifecycle.
- Add Criterion/IAI benchmarks before claiming performance.
