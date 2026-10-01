# API Review: HPC and ontology/game preview integration

## Problem and affected roots

Tracks 46–55 extend `kairo-ecs-pdes`, `kairo-ecs-mpi`, `kairo-ecs-grpc`, `kairo-ecs-core`, `kairo-ecs-state`, `kairo-ecs-ffi`, `kairo-ecs-arrow`, `kairo-ecs-gpu` and `kairo-ecs-fmi`. Tracks 56–61 add `kairo-ecs-game-ontology` and graph/normal-form/extensive-form extensions to `kairo-ecs-game-theory`. Their precise owned APIs, feature gates and incomplete capabilities are described in each numbered track specification and test matrix.

## Proposed API and compatibility

| Surface | Change | Compatibility and maturity |
|---|---|---|
| Rust | Feature-gated HPC contract primitives, ontology IR/code generation, graph/game prototype modules | Preview. Existing sequential core and logical time contracts retained. |
| Rust FMI extraction | Existing extraction signature now rejects an existing destination | Security correction; use a fresh output path and a caller-controlled parent. |
| Rust conservative PDES | Every declared peer constrains the initial safe-time frontier | Correctness correction; bootstrap explicit peer null-message promises. |
| Rust TimeWarp | Rebuild retains committed state below GVT | Correctness correction; cancellation at GVT preserves committed effects. |
| C ABI | NUMA/zero-copy layout contract metadata | No changed exported C function signatures or ownership transfer. |
| Arrow and host bindings | Existing shared field and binding contracts | No changed published schema, package root or host API signature. |

## Ownership, errors and threading

FMU extraction owns a newly created destination; existing destinations fail through the existing I/O error model. This prevents following preexisting destination symlinks. Existing transport and rollback error types remain in place. Hardware/thread/parallel backend support is limited by each track's documented blocked paths; no live backend acceptance follows from local tests.

## Determinism and conformance

Existing deterministic scheduler/binding fixtures remain unchanged. Added fixtures cover ontology parsing and generated Rust, feature-isolated graph/game solvers, FMU traversal/symlink rejection, conservative peer frontiers, and rollback state at the fossil boundary. The targeted PDES/FMI suites exercise the corrected contracts; the complete workspace gate validates their integration.

## Red-team response and decision

The integration review found and repaired FMU symlink escape, incomplete conservative peer bounds and loss of committed rollback state. ADR 0006 records those contract decisions. Accept prototype integration with the existing incomplete/live-runtime gates preserved. Beta/stable publication requires the track-specific API and release acceptance reviews; this document does not grant publication authority.
