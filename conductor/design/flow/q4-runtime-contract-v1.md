# Q4 experimental runtime contract v1

**Status:** Accepted bounded internal experimental contract review by the
CareOps coordinator acting in Track 01B/03/25 review roles on 2026-10-03.
The concrete preview DTO, budget architecture, fail-stop semantics and default
below are accepted for test-first packet preparation. Source implementation
requires its own reviewed packet. No external maintainer signature, full Q4
acceptance or release approval is claimed.

**Qualified baseline:** Kairos
`237c2c08d04cf038ed324c272ffbdb63dae6f57b`, with CareOps parent
`2b0a27425c1a9d811107f2ce8ef106aaf4861f7f`.
Current parent `8725b00c9bdbcee07ed07bbf2a8e895bb037875b` descends
from that baseline and adds parameter schema/example CLI tests only. Its queue
spec SHA256 remains
`0809d5754ca44ed5d57a8225d6a9641ea587185c7a2d2bfb7f1c65309e3e7943`.
This ancestry refresh does not qualify new Q4 runtime behavior.

## Scope and owners

Track 01B owns scheduler preview and its cancellation/order tests. Track 03
owns Flow planning, budget state, typed continuations, domain adapters and
synthetic workflows. Track 25 reviews concrete public symbols and compatibility.
Track 04 owns lifecycle Arrow schema and encoding; its field applicability
matrix is a separate gate. Track 22 owns run manifests and portable checkpoints.
No core, DES, ABM, Arrow, manifest or binding source is changed by this draft.

Read this with the CareOps queue spec sections 6–9, its
`conductor/design/queue/q0.3-lifecycle-join-decision-20260930.md`, and this
repository's `conductor/design/queue/q0.3-resource-lifecycle-sidecar-proposal-20260930.md`.
The Q0/Q3 contracts remain authoritative. Existing FIFO Resource, DESContext,
ABMContext and Q3 WorkHandlers APIs retain their behavior.

## ADR disposition: preview before execution

Decision: use a scheduler preview, followed by pure Flow transaction planning
and aggregate admission, before consuming the event. This preserves the
scheduler's pending-event, clock and dispatch-counter semantics on budget
failure. A consumed event held outside the scheduler would change those
semantics and is not the selected architecture.

The coordinator independently reviewed the actual QueueEntry ordering,
pruning and step implementation and the Track 01/25 ownership contracts.
This bounded internal Track 01B/03/25 review accepts the copied preview DTO
and budget semantics as an additive experimental Rust-only surface. It changes
no event ordering, time representation, handle encoding, ABI, facade, binding
or RNG behavior. Existing experimental development/release holds remain;
implementation and conformance evidence are still required. This is an internal
coordinator disposition, not an external maintainer signature or a declaration
that the whole Q4 phase or stable release surface is accepted.

### Exact preview surface proposed for review

Located in kairo-ecs-core, without a types-crate change:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScheduledEventPreview {
    pub id: EventId,
    pub at: SimTime,
    pub priority: i32,
    pub sequence: u64,
    pub entity: Option<EntityId>,
    pub kind: EventKind,
}
// inherent Scheduler method
pub fn peek_next(&mut self) -> Option<ScheduledEventPreview>;
```

The DTO is a projection of the next live queue entry, not a dispatched event.
The method may prune cancelled heap entries using the existing private pruning
logic. It does not change now, scheduled/dispatched/cancelled counters, live
pending membership, IDs or insertion sequence. Repeated preview without
schedule/cancel/dispatch returns identical data. It returns None exactly when
no live event remains. It never exposes a heap reference.

Order remains (time, priority, sequence). A subsequent schedule or cancel may
change the next preview. Flow holds exclusive access and performs neither
operation nor callbacks between its final preview and corresponding step.
That step must return exactly the previewed event. No automatic retries hide
an invariant mismatch. Preview adds no global event-kind allocation.
No public facade/FFI preview is implied.

Required Track 01 fixtures: cancelled heads and future entries, repeated
preview, unchanged stats/time/live pending count, priority/sequence extremes,
preview then cancel, preview then earlier schedule, and preview then exact
step identity. Existing scheduler ordering and cancellation regressions remain.

## Configuration and budget surface proposed for review

```rust
pub struct FlowConfig {
    pub max_same_tick_flow_transitions: std::num::NonZeroU64,
}
// new()/Default use 100_000; with_config uses an explicit pre-run setting.
pub fn with_config(config: FlowConfig) -> FlowRuntime;
```

The finite positive default is **100,000**, a safety cap, not evidence of
clinical validity or a performance tuning result. Zero is unrepresentable in
this Rust constructor; external configuration parsers must reject zero.
Configuration is immutable for that run. Manifest integration must record
the actual setting through Track 22's reviewed surface before claiming
manifest compliance. Merely storing FlowConfig is insufficient.

A separate read-only budget inspection surface must expose the counted tick
(if one exists), consumed count, limit and optional halt details. Halt details
include the blocked preview identity/time and the transaction's required
cost. Exact inspection DTO names are gated with the implementation packet;
no mutable scheduler or authoritative ECS values are exposed.

### Accounting

Count one for each committed lifecycle transition and one for each delivered
Flow continuation notification at its simulation tick. Counts persist across
step/run_for calls. max_events is an independent per-call dispatch bound.
A notification delivered to its live typed handler counts once even when its
later command batch is rejected. A stale/dead notification with no callback
delivery costs zero. Stale internal completion/deadline tokens cost zero.
Raw domain event dispatch itself costs zero; any Flow commands admitted by
its handler count their transitions only on later command dispatches.

When an event's tick differs from the previous counted tick, normal progress
starts a new count. Reaching the limit exactly is permitted, including later
zero-cost dispatches. A naturally next later-tick event resets the count.
A blocked same-tick event cannot be bypassed to produce that reset.

### Planning, commitment and failure

At preview.at, stage due boundaries, explicit operation, arbitration, work
accounting, lifecycle snapshots, cleanup, token scheduling and checked counters
using an explicit effective timestamp. Scheduler.now has not advanced yet.
Do not use its old now accidentally in the planner.

Preflight arithmetic and exact aggregate lifecycle cost before any state write,
factory, token scheduling or dispatch. A valid semantic rejection can still
commit its independent due boundaries, as Q3 specifies. Budget rejection
commits none of the staged plan, including independent boundaries.
After successful preflight, consume the exact previewed event, then perform
the already reserved/infallible commitment. Factories retain Q3's after-
arithmetic-preflight ordering and reuse original sampled duration.
Continuation delivery preflight occurs before consuming its event, removing
its notification or invoking its handler.

On exceeding the budget, return structured
`SameTickBudgetExceeded { at_ticks, limit }` and retain the entire failing
event in Scheduler's live pending set and all command/notification metadata.
No clock advancement, dispatch count, row, allocation, cleanup, factory or
callback mutation occurs. Record a permanent run halt with inspectable pending
work. Runtime.now may precede the blocked tick; the error identifies the latter.

Every further step/run_for returns the same halt without executing anything.
max_events=0 does not reset or bypass it. Mutating ingress/setup/configuration
after halt returns a structured halted-run error; read-only inspection remains.
There is no skip, cancellation, rescheduling to a later tick, counter reset,
clock jump or budget increase for this halted run. A larger-budget replay is
a separately configured new run, not portable checkpoint recovery.

Fixtures must distinguish normal later-tick reset from failure: one test reaches
the limit legally then advances; another blocks a positive-cost head and
proves its later event never dispatches. Also test aggregate multirow overflow,
pending identity/counters, repeat calls, zero-cost stale events, notification
counting and no factory/cleanup/context mutation on budget failure.

## Callback and domain ingress disposition

Existing `WorkHandlers<C>` with `fn(&mut C, &WorkProgress)` remains unchanged.
A new continuation registration may provide mutable live owned context,
immutable captured causal snapshot and a restricted command sink. It must
not expose &mut FlowRuntime, step/run, direct scheduler access, authoritative
Flow component mutation or synchronous arbitration.

New callback batch types and registration names are **not frozen here**.
Before fixtures/implementation, separately review the exact command variants,
ticket/receipt types, typed ownership, result/error channel and completion hook.
The accepted semantic disposition is:

- Delivery is once-only after budget preflight, in scheduler notification order.
- Context is live owned C; progress/origin remains the captured Q3 snapshot.
- Sink commands are owned specifications, not fabricated RequestId/EventId.
  Batch-local tickets resolve through actual admission receipts.
- Validate/reserve the entire batch in emission order against a staged admission
  view, including duplicate work association, handle/type/time checks,
  reserved kinds, entity and scheduling arithmetic. Admit all or reject all.
- Later command dispatch gets its own causal EventId and normal boundary rules.
  Admission is not a promise that execution-time state remains valid.
- Rejected batch admits no command, ID, association or partial reservation.
  The callback remains delivered; its context mutations remain. Expose rejection
  separately from lifecycle state commitment. Do not replay the callback or
  claim rollback of arbitrary context mutation.
- Panic recovery/replay is not promised. New fallible callback behavior requires
  the same delivered-once and nonrollback context disposition.
- Completion continuation must support staged workflow composition without
  another lifecycle row merely for delivery.
- General domain hooks are a separate additive registration. Reject 4000–4003
  at every public ingress and explicitly report unregistered domain events.

Initial callback command scope should reuse already-created work and existing
Flow operations. Typed work/entity creation inside a callback is separately
gated; it must not be smuggled through mutable registry access.

## Shared adapters and lifecycle telemetry barriers

DES and ABM adapters borrow exactly this runtime's scheduler time, world and
component registry. Existing ABMContext owns a competing world and is not the
new adapter. Preserve its legacy standalone behavior. Borrowing/behavior RNG
and entity access signatures require a separate concrete API review; no new
duration draw, wall-clock seed or domain policy in core.

Lifecycle export snapshots must be captured per staged transition, not read
from final dispatch state. Preserve event_log.v1, 12-byte IDs, unsigned
16-byte little-endian ticks and contiguous UInt32 causal ordinals. Notifications
reference their origin and create no delivery lifecycle row. Track 04 must
freeze applicability/nullability/reasons and roundtrip its schema separately.
Generic C1 transport and C4 calibration paths are outside this work.

## Next fixture packets and acceptance boundaries

Following the bounded internal review above, prepare a Track 01 preview fixture packet
and a Track 03 budget planner fixture packet. They can own separate test files;
one writer performs shared FlowRuntime integration. Callback batch fixtures
wait for their concrete type contract. ABM adapters wait for shared borrowing
review; Arrow encoding waits for committed snapshot/schema review.

No Q4 checkbox is completed by this draft. No runtime/native/hosted test pass
is claimed. Q3's qualified runtime, seeded model and legacy fixtures remain
regression inputs. Portable checkpoints, complete Track 03 release and binding
promotion remain outside this contract.
