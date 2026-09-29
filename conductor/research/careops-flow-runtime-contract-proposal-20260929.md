# Proposed FlowRuntime contract for Kairos

Status: owner-approved Q0.1 architecture direction; exact public API and implementation remain gated.
Date: 2026-09-29
Scope: Q0.1 runtime ownership, opaque handles, errors, and supported-use limits.

## Decision

Add an experimental, Rust-native `FlowRuntime` to `kairo-ecs-des`. One runtime owns exactly one private `Scheduler`, `World`, and `ComponentRegistry`. Resource, claim, work, lease, and interruption state belongs to that shared runtime, so its DES events and Flow-facing ABM work observe one simulation clock and one ECS world.

Keep the existing `DESContext`, `Resource`, `ABMContext`, `BehaviorContext`, `AgentBehavior`, and `BehaviorSimulation` APIs and their observable behavior unchanged. This proposal adds a separate path; it does not migrate or retrofit legacy users. It leaves scheduler ordering and RNG derivation unchanged. DES owns deterministic dispatch and its registered handler boundary. The separate Flow-specific adapter lives in `kairo-ecs-abm` and depends on DES; DES must not depend on ABM. The adapter implements the DES-defined behavior hook. Existing standalone ABM APIs and behavior remain unchanged.

The public Flow handles are opaque newtypes whose fields and constructors remain private:

| Handle | Identity and validity |
|---|---|
| `ResourceId` | Wraps a generational `EntityId`; valid only while that exact resource entity remains live. |
| `ClaimId` | Wraps the request entity identity and identifies its retained terminal outcome. |
| `WorkId` | Wraps the work entity identity for one registered interruptible unit of work. |
| `LeaseId` | Identifies an active allocation using its claim identity and a checked lease revision. A subsequent grant has a distinct revision. |

Callers create and operate on these identities through `FlowRuntime` methods. They cannot construct handles from arbitrary entity IDs or mutate internal queue, allocation, work, scheduler, world, or component-registry state. The runtime may expose read-only snapshots or queries, but not mutable core stores. Every command validates handle generation/liveness and the relevant lifecycle state before changing state. Time-bearing commands reject a requested time earlier than the runtime's current time before calling the core scheduler.

`FlowError` is the common typed boundary for expected command failures. Its variants should distinguish stale handles, unknown resources, terminal claims/work, invalid time, capacity conflicts, configured limits, and invalid command/builder state. Errors must not silently repair invalid state or partially apply a command. The final public variant names and payloads remain subject to Track 25 API review.

## Q0.1 owner disposition and clarified boundaries

On 2026-09-29 the Kairos owner directed adoption of the four recommended Q0.1
dispositions: (1) private FlowRuntime with facade-only mutations and cumulative
`u32::MAX` operation caps; (2) DES-owned shared runtime/dispatcher with an ABM
adapter depending on DES; (3) experimental API-root registration and compatibility
review for both DES and ABM because the adapter adds public ABM symbols; and (4)
typed, owned, in-memory continuation with portable checkpointing deferred to
Track 22. The owner direction does not remove Q0.2/Q0.3, D2, exact-symbol review,
or release gates.

The cap proof assumes a new Scheduler and World whose counters/generations start
at zero; a private runtime that never exposes mutable core/state objects; and no
schedule/create/despawn mutator path outside the checked facade. `u32::MAX` is an
overflow envelope, not an allocation, memory, or throughput guarantee. Flow-owned
cleanup covers only engine-owned component types. Before despawning, the runtime
must remove the target entity's typed components and all reverse references to it
in queues, allocations, claims, leases, and work records. Those internal value
types must not introduce user-defined panicking destructors. If cleanup or
`World::despawn` fails, the runtime must not report success; rollback after a
panic is not promised.

DES owns event dispatch and handler registration. The ABM adapter implements the
Flow-specific hook, uses typed read-only queries, and returns buffered commands;
DES validates and applies them at a deterministic dispatch boundary. No handler
may expose mutable scheduler/world/registry state. Query/application order must
not depend on `HashMap` or dense component-store iteration. Q0.2/Q0.3 must still
freeze the callback and RNG identity lifecycle, command-buffer atomicity and
rejection behavior, event-kind allocation, and notification order. These details
are not public signatures approved by this Q0.1 disposition.

Both `crates/kairo-ecs-des` and `crates/kairo-ecs-abm` are now registered as
experimental protected Rust API roots. The owner-approved design review is
recorded at `docs/design/api-reviews/flow-runtime-q0.1.md`. It accepts the
architecture and surface classification only; release remains held until exact
symbols are reviewed against the frozen semantics and implementation evidence.

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

FlowRuntime registers typed cleanup hooks for every component type that its facade inserts. Before despawning an entity, it removes Flow queue/work/claim/lease state and invokes those hooks to remove that entity from each registered component store. The cleanup guarantee applies only to engine-owned Flow component types and reverse references and only when the cleanup hooks complete and the underlying `World::despawn` succeeds. The runtime must not provide a mutable registry handle or insertion route for unmanaged components. These internal value types must not run user-defined destructors. This is not a panic-proof transactional or rollback guarantee: if cleanup or despawn fails or panics, the runtime must not report success.

The limits intentionally constrain supported Flow use rather than changing the guarantees of Scheduler or World for other callers. If broader checked-overflow guarantees are required, Track 01 must separately design additive core/state contracts and tests.

## Compatibility boundary

This is an additive experimental Rust proposal rooted in `kairo-ecs-des`, with a Flow-specific adapter in `kairo-ecs-abm`. Both exact crate roots are registered in the protected-surface inventory. It preserves legacy DES/ABM public APIs and behavior, does not alter core event ordering or RNG stream derivation, and does not change Arrow schemas, C ABI, WASM/host bindings, or Cargo manifests in this planning leaf. Queue priority remains a resource-allocation concern, separate from scheduler event priority. The concrete Flow symbols remain under release hold until Q0.2/Q0.3 and implementation-level review.

The Q0.1 owner-approved design disposition registers the exact `crates/kairo-ecs-des` and `crates/kairo-ecs-abm` roots as experimental and aligns the compatibility artifacts. This is not a concrete-symbol API review or release approval. Exact symbols remain under release hold pending Q0.2/Q0.3 contracts, implementation-level Track 25 review, and tests. Existing Kairos APIs remain available regardless of the future Flow API's experimental classification.

The owner-approved Track 03 direction places the adapter in `kairo-ecs-abm` with an acyclic dependency on DES. DES owns deterministic dispatch and handler registration; the adapter uses restricted typed queries and buffered checked commands, preserving standalone `BehaviorSimulation`. Q0.2/Q0.3 still freeze exact callback signatures, per-agent RNG lifecycle, event IDs, command validation/atomicity, and transition order.

Continuation context serialization and snapshot ownership are deferred to Q0.1.codec. This proposal only requires any in-memory continuation state used by later queue work to be owned by the shared runtime; it does not specify a portable codec or snapshot format.

## Remaining Q0.2/Q0.3 contract questions

1. Q0.2 must decide claim terminal-record retention and lease-revision lifecycle. Claim disposal is allowed only after terminal state and when no lease remains active.
2. Q0.2 must specify atomicity and failure behavior for state transitions, exact same-tick boundaries, and typed cleanup of reverse references.
3. Q0.3 must freeze event-kind IDs, callback registration/dispatch identity, per-agent RNG lifecycle, buffered-command validation/application, lifecycle record schema, and transition order.
4. Concrete method/error signatures must be reviewed against the two registered roots before implementation is merged. Portable checkpoint semantics remain with Track 22.

The Kairos owner approved the Q0.1 architecture direction above. These remaining design details must be settled before implementation; the approval does not authorize broader core/state API changes, host bindings, or release.
