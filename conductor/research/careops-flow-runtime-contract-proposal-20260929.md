# Proposed FlowRuntime contract for Kairos

Status: proposal for owner and track review; not an approved API or implementation.
Date: 2026-09-29
Scope: Q0.1 runtime ownership, opaque handles, errors, and supported-use limits.

## Decision

Add an experimental, Rust-native `FlowRuntime` to `kairo-ecs-des`. One runtime owns exactly one private `Scheduler`, `World`, and `ComponentRegistry`. Resource, claim, work, lease, and interruption state belongs to that shared runtime, so its DES events and Flow-facing ABM work observe one simulation clock and one ECS world.

Keep the existing `DESContext`, `Resource`, `ABMContext`, `BehaviorContext`, `AgentBehavior`, and `BehaviorSimulation` APIs and their observable behavior unchanged. This proposal adds a separate path; it does not migrate or retrofit legacy users. It leaves scheduler ordering and RNG derivation unchanged. It does not add a dependency from DES to ABM or choose another crate boundary for the behavior adapter.

The public Flow handles are opaque newtypes whose fields and constructors remain private:

| Handle | Identity and validity |
|---|---|
| `ResourceId` | Wraps a generational `EntityId`; valid only while that exact resource entity remains live. |
| `ClaimId` | Wraps the request entity identity and identifies its retained terminal outcome. |
| `WorkId` | Wraps the work entity identity for one registered interruptible unit of work. |
| `LeaseId` | Identifies an active allocation using its claim identity and a checked lease revision. A subsequent grant has a distinct revision. |

Callers create and operate on these identities through `FlowRuntime` methods. They cannot construct handles from arbitrary entity IDs or mutate internal queue, allocation, work, scheduler, world, or component-registry state. The runtime may expose read-only snapshots or queries, but not mutable core stores. Every command validates handle generation/liveness and the relevant lifecycle state before changing state. Time-bearing commands reject a requested time earlier than the runtime's current time before calling the core scheduler.

`FlowError` is the common typed boundary for expected command failures. Its variants should distinguish stale handles, unknown resources, terminal claims/work, invalid time, capacity conflicts, configured limits, and invalid command/builder state. Errors must not silently repair invalid state or partially apply a command. The final public variant names and payloads remain subject to Track 25 API review.

## Handles and errors

The runtime is the authority for handle creation and validation. Resource and work entities are created through it; claim handles are returned when a request is accepted; lease handles are returned when allocation succeeds. The caller treats all handles as opaque values and may retain them for later operations, but every operation revalidates the underlying generational identity. A despawned entity's old handle is stale even if its slot is reused.

A claim's terminal record remains addressable through its `ClaimId` for the lifetime of that claim entity, so callers can distinguish completed, cancelled, timed out, or otherwise terminal requests from unknown or stale identities. Whether terminal records are retained until explicit disposal or runtime teardown, and the detailed terminal-state set, must be fixed with Q0.2 semantics before implementation.

`LeaseId` adds a monotonically checked revision to the claim identity. Releasing or invalidating a lease consumes that revision; a later allocation cannot make an old lease valid again. Revision exhaustion returns a typed limit error before changing the allocation. Revision width and the precise lifecycle transitions should be confirmed alongside Q0.2's allocation semantics.

All Flow commands validate before mutation. In particular, past-time requests are rejected before `Scheduler::schedule`; stale IDs are rejected before component access; capacity and terminal-state checks precede queue/allocation edits; and configured operation limits are checked before invoking the underlying mutator. The API must not expose mutable references that let callers bypass those checks.

## Overflow and cleanup

The current scheduler counters and event generation, and the world's entity generation, are not globally checked against overflow. This proposal does not change those core/state APIs and does not claim they are globally overflow-safe. Instead, FlowRuntime offers a supported-use envelope with lifetime counters starting at zero for each new runtime:

- at most `u32::MAX` successfully scheduled events;
- at most `u32::MAX` successfully created entities through the runtime; and
- at most `u32::MAX` successful entity despawns through the runtime.

Each counter is maintained by FlowRuntime using checked arithmetic. When the next operation would exceed its limit, the runtime returns the configured-limit error before calling the underlying scheduler/world operation or changing Flow state. Counters are cumulative for the runtime lifetime and are not reset when an entity is despawned or an event is cancelled. This conservative total-despawn bound ensures no entity generation can wrap due to Flow-mediated despawns in a runtime whose world starts with fresh generations. All relevant mutations, including resource/claim/work creation and cleanup, must go through this facade for the limit to apply.

FlowRuntime registers typed cleanup hooks for every component type that its facade inserts. Before despawning an entity, it removes Flow queue/work/claim/lease state and invokes those hooks to remove that entity from each registered component store. The cleanup guarantee applies when the in-memory cleanup hooks complete successfully and the underlying `World::despawn` succeeds. This is not a panic-proof transactional or rollback guarantee: if cleanup or despawn fails or panics, the runtime must not report a successful despawn, and recovery semantics require owner review. Components inserted outside the Flow facade are outside this cleanup guarantee; the runtime must not provide a mutable registry handle that enables such insertion.

The limits intentionally constrain supported Flow use rather than changing the guarantees of Scheduler or World for other callers. If broader checked-overflow guarantees are required, Track 01 must separately design additive core/state contracts and tests.

## Compatibility boundary

This is an additive experimental Rust proposal located in the existing `kairo-ecs-des` crate. It preserves all legacy DES and ABM public APIs and behavior, does not alter core event ordering or RNG stream derivation, and does not change Arrow schemas, C ABI, WASM/host bindings, or Cargo manifests in this planning leaf. Queue priority remains a resource-allocation concern, separate from scheduler event priority.

No compatibility or release approval is implied. Track 25 must classify the exact `kairo-ecs-des` public root in the protected-surface inventory and complete its API review before implementation is treated as reviewed for release. Track 01 must review the supported-use overflow and cleanup boundary. Existing Kairos APIs remain available regardless of the future Flow API's experimental classification.

The exact Track 03 behavior-adapter dependency and dispatch route remain open for review. Track 03 should determine how a Flow-specific adapter reads the shared runtime through restricted queries, submits checked commands at deterministic dispatch boundaries, and avoids the separate `BehaviorSimulation` context. This proposal does not choose whether that adapter lives in DES, ABM, a third crate, or a particular event-kind dispatch mechanism.

Continuation context serialization and snapshot ownership are deferred to Q0.1.codec. This proposal only requires any in-memory continuation state used by later queue work to be owned by the shared runtime; it does not specify a portable codec or snapshot format.

## Open owner review

1. Does Kairos accept an additive experimental `FlowRuntime` in `kairo-ecs-des` with one private scheduler, world, and component registry while preserving every legacy DES/ABM API and behavior?
2. Does Track 01 accept the per-runtime `u32::MAX` lifetime caps as the supported-use boundary for existing unchecked scheduler/world counters, with broader overflow guarantees deferred to separate core/state work?
3. Does Track 01 agree that typed cleanup hooks for every facade-inserted component, plus queue/work cleanup before despawn, are sufficient for Flow-mediated entity cleanup?
4. Which exact behavior-adapter dependency and deterministic dispatch route should Track 03 select? This remains unresolved here by design.
5. Which terminal-claim retention and lease-revision lifecycle details should Q0.2 freeze before code implementation? Q0.2 should also require claim disposal to be allowed only after the claim is terminal, and forbid disposal while any lease for that claim remains active.
6. Which exact experimental/protected-surface classification and review evidence does Track 25 require for this API root?
7. Confirm that continuation codec and snapshot ownership remain deferred to Q0.1.codec.

Until these owners review the relevant questions, this document is a contract proposal only. It grants no authority to modify public APIs, core/state behavior, compatibility policy, or release status.
