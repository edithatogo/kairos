# Migration notes

## 7 November 2026: Rust-only simulation execution

[ADR 0010](../adr/0010-rust-only-execution.md) ends support for independent
host-language cores immediately. There is no parallel support period. Host
languages may orchestrate experiments and analyse outputs; scheduling, state,
RNG and simulation model execution belong to Rust native code or Rust-compiled
WebAssembly.

| Previous use | Required action |
| --- | --- |
| Python `Scheduler` preview | Build the matching Rust C ABI library and use `NativeEngine` for its exposed operations. Set the explicit absolute library path. Move other execution into Rust until a native adapter supports it. |
| Python demonstration `VecEnv` or PettingZoo environment | Stop using synthetic observations as simulation results. These constructors are unsupported until a Rust model adapter is implemented and qualified. |
| TypeScript `createSchedulerFacade` or `TimeTravelInspectorClient` | Generate the Rust/Wasm module and use `WasmBridgeLoader.loadWasmEngine` with its module URL. Do not infer time-travel restoration from snapshot-shaped metadata. |
| R, Go or C# managed scheduler preview | Stop executing the host scheduler. Use the qualified native ABI surface where available, or the Rust API. Unsupported managed constructors return explicit errors. |
| Julia or other metadata-only native probe | Treat metadata, DTO conversion and library-path checks as their actual scope. Require installed Rust execution before claiming an engine is available. |

Start with [installation](../install.md), the
[Python tutorial](../tutorials/python-getting-started.md) or
[Wasm tutorial](../tutorials/wasm-getting-started.md). Retain source revision,
native artefact checksum, ABI, enabled features and runtime version in new
acceptance receipts. Old host-only golden outputs are not native conformance
proof; compare against independently specified Rust semantics and fixtures.

Applications such as `careops-sim` should consume Kairos as a pinned dependency
through public APIs and versioned data contracts. Keep domain policy and model
validation in the application. See the
[consumer-contract track](../../conductor/tracks/114-dependency-and-consumer-contracts/plan.md).

## CareOps staged allocation: `FlowRuntime` to `StagedResourceRuntime`

The removed `kairo-ecs-des::FlowRuntime` is not restored. CareOps consumers that
used its narrow capacity, request-state, lease-revision, release and
queue-promotion behaviour must migrate call sites to the experimental
`StagedResourceRuntime` API. This source migration does not provide compatibility
for callbacks, checkpoints, fidelity modes, preemption or asynchronous
combinators formerly associated with the broader runtime.

The migration remains alpha until API review, conformance fixtures and consumer
acceptance pass. No parent dependency pin is implied by this note.
