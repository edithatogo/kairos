# KairoECS Crates

This directory is the implementation root for the Rust workspace.

Initial crates are deliberately narrow:

- `kairo-ecs-types`: shared public types and DTOs.
- `kairo-ecs-core`: deterministic event scheduler skeleton.
- `kairo-ecs-state`: entity lifecycle skeleton.
- `kairo-ecs-rng`: deterministic seed/stream skeleton.
- `kairo-ecs-ffi`: stable C ABI bridge skeleton.
- `kairo-ecs-uniffi`: UniFFI wrapper root anchored to the C ABI.
- `kairo-ecs-diplomat`: Diplomat wrapper root anchored to the C ABI.
- `kairo-ecs-bench`: benchmark helper crate for core and bridge scenarios.

Additional crates listed in `conductor/tech-stack.md` should be added only when their owning track reaches implementation.
