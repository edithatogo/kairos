# Migration notes

## CareOps staged allocation: `FlowRuntime` to `StagedResourceRuntime`

The removed `kairo-ecs-des::FlowRuntime` is not restored. CareOps consumers that used its narrow capacity, request-state, lease-revision, release and queue-promotion behavior must migrate call sites to the new experimental `StagedResourceRuntime` API. This is a source migration and does not provide compatibility for callbacks, checkpoints, fidelity modes, preemption or asynchronous combinators formerly associated with the broader runtime.

The migration is available only at alpha until the API review, conformance fixture and consumer acceptance gates pass. No parent dependency pin is implied by this note.
