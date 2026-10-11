# ADR: experimental resource grant eligibility gates

Status: internally reviewed design contract (root and GPT-6.1-sol in Track03
owner-review role), not standalone implementation authority or API acceptance.
Base: 6d733f489225da4bf6ceec2673b02369c7918bbe. Rust/Cargo 1.99.0 only.
CareOps E2.1 acceptance and a reviewed bounded packet precede implementation.
The coordinator acts within the existing single-maintainer owner-review workflow;
no external maintainer approval or hosted qualification is claimed.

## Problem and ownership

Calendar closure must prevent new grants, preemption and suspended resumption
while already active work drains. Capacity zero cannot express this when active
claims exist. Post-allocation callbacks are too late; cancel/resubmit corrupts
Suspend identity and accounting. A single FlowRuntime remains authoritative.
ED skills, zones, calendars and urgency/ASCII domain selection stay outside the
reusable engine. Track03 owns DES/ABM; Track22/calibration owns checkpoint consumers.
No core scheduler, bindings, Arrow schema or package-root rename is needed.
The experimental public Rust surface extends through the existing `pub use
flow::*` re-export; no explicit lib.rs edit is planned.

## Proposed Rust contract

```rust
pub fn schedule_resource_grant_enabled(
    &mut self, resource: ResourceId, enabled: bool,
    at: SimTime, scheduler_priority: i32,
) -> Result<EventId, ResourceGrantAdmissionError>;
pub fn resource_grant_enabled(&self, resource: ResourceId)
    -> Result<bool, FlowError>;
```

The accessor returns the committed flag, not a predicted future effective flag.
Resources start enabled. Private pending boundaries join resource/time to an
actual scheduled gate command EventId. No new callback API or second clock.
Add a separate `ResourceGrantAdmissionError` enum with `Flow(FlowError)`,
`DuplicateBoundary` and `FrontierReached` variants, deriving Clone, Copy, Debug,
Eq, PartialEq and thiserror::Error. Flow wraps its source and implements From.
Existing FlowError variants and its exhaustive V1 error wire remain unchanged.
Runtime dispatch still uses existing FlowError categories; the new type reports
only gate admission. No nested V1 error tags are added.

Admission validation order: resource identity, time earlier than current now,
started-frontier rule, duplicate resource/time, pending removal, scheduler and
command/index counter budgets. Time earlier than now returns Flow(PastCommand);
time at/before a started frontier returns FrontierReached.
Duplicate same-resource/time returns DuplicateBoundary even if identical.
Queued removal returns Flow(ResourceInUse); other existing failures wrap Flow.
Validate every recoverable
failure before mutation; invalid admission preserves scheduler counters, next
EventId, flags, commands, pending index and queues. Allocation failure/panic is
not promised recoverable. Gate index publication follows successful preflight.

An explicit optional started frontier distinguishes initial zero from executed
zero. It advances only with committed scheduler dispatch, including a command
that subsequently receives a semantic rejection. A precommit budget rejection
must not advance it. Existing persisted scheduler dispatch counters may provide
a verified equivalent; exact representation is private, semantics are fixed.

## Allocation and same-tick ordering

Before allocation on R at T, settle its due completions. Treat R as closed if
its candidate staged flag AFTER the explicit command is disabled OR an accepted
pending Disable(R,T) exists, independent of command priority/insertion order. Closed means no free grant,
preemption or resume. Completion, release, cancellation and deadline handling
retain existing behavior. No transient claim may be created then undone.

The disable still dispatches as a real command. Its pending index remains visible
through planning, including its own disable lookahead. Validate the EventId join
before dispatch, then remove the entry only after committed scheduler dispatch
with command consumption (including a dispatched semantic rejection). A
precommit budget halt does not remove it. Enable is never applied
by lookahead: its command settles completions, stages the enabled flag and wakes the
queue using that candidate state before commit. Both changes and allocation
participate in existing staged dispatch and rollback semantics. Existing dispatch
budget-halt metadata may change on a precommit failure; flags, pending gate index
and claims do not. Exact all-state atomicity above applies to gate admission.
Resource inspection never invents a future enable.

The parent must predeclare boundaries before execution reaches their tick and
admit all ready-at-T domain work before an enable allocates. Parent phases may
use notifications0, submissions-1, enable10; these are not engine-wide reserved
priorities. Unknown future same-tick external ingress cannot retrospectively
reorder grants. Normalize adjacent calendar intervals to one net boundary.
No configured gates preserves all legacy event ordering and fixture outputs.

## Resource lifetime

Reject remove_resource admission while any gate command for the resource is
pending. Reject gate admission while Remove is queued. Repeat the invariant at
removal dispatch. No implicit cancellation or silent toggle loss. Disabled idle
resources with no pending toggles may be removed; reused generational identities
start enabled and inherit no old index state. A stale resource is InvalidResource.

## Checkpoint compatibility and handoff

V1 DTO/wire/tag layout and golden bytes remain unchanged. From the first interim
runtime packet, V1 capture rejects disabled resources OR any pending gate command
through FlowCheckpointError::InvalidState; no lossy image may escape before V2.
V1 restore produces all-enabled/no-pending state and derives initial/started
frontier from persisted scheduler dispatch state.

Explicit V2 must preserve flags, pending command/event IDs, times/priorities and
frontier. Validate resource references and one-to-one command/scheduler/index
joins before publishing restored state. V2 wire/DTO details need a separate
reviewed checkpoint contract. Runtime interface may freeze after focused tests;
combined acceptance waits for V2 and calibration/Track22 consumer qualification.

Affected consumers: calibration c2_portable_checkpoint.rs,
flow_bridge/checkpoint_wire.rs, flow_bridge.rs, shadow_native.rs and c5_native.rs.
Use lossless version-aware views; old readers cannot read new inner V2 bytes.
An unchanged outer C2 envelope is not backward reader compatibility. ABM transit
context still participates in coherent-cut validation; no separate clock appears.

## Compatibility and migration assessment

| Surface | Impact | Status |
| --- | --- | --- |
| Experimental Rust Flow | Add methods and separate admission error type; existing FlowError unchanged | Preview, not stable promotion |
| Legacy no-gate Flow behavior | Preserve exact ordering and outputs | Regression qualification required |
| V1 checkpoint bytes/read | Preserve layout, tags and golden fixtures | Disabled/pending state capture rejects explicitly |
| V2 checkpoint/consumers | Explicit new format and version-aware restore | Separate dependent packet |
| C ABI/host languages/Arrow | No API or schema change in this packet | No new support claim |

The methods use exclusive &mut self for mutation and shared &self for inspection.
No new unsafe code, thread-safety promise, cloned runtime or ownership transfer.
No stable error-message compatibility; variants carry programmatic categories.
No release, publication, remote push or historical Track03 completion change is
implied by this proposed extension.

## Native oracles and red-team objections

- Completion3 and closure[3,5): completion3, no grant3, grant5 in either event order.
- Low10 from1, urgent2 ready3, closed3-5: low1-5, urgent5-7, low7-13; one sample.
- Suspended low at4, urgent finishes6, closed6-8: resume8, never6.
- Parent admits urgent5 before enable5: urgent first without gratuitous resume.
- EnableH can grant zero observed work; no invented completion.
- Closure blocks allocation even with i32::MIN submission priority; duplicate
  boundary and adjacent normalized calendars do not create transient claims.
- Remove/toggle admission in both orders, disabled-idle removal, generational reuse.
- Invalid identity/time/frontier/duplicate/budget admission leaves exact state and
  next valid EventId equal to untouched control. Initial zero vs started zero and
  semantic-rejection vs precommit-budget-rejection are separate cases.
- V1 capture disabled/pending rejects immediately; legacy V1 bytes/read still pass.
  Later V2 pending/disabled restore reproduces full uninterrupted history, with
  malformed references/tags/versions rejected atomically.

Objection: lookahead mutates scheduling semantics. Response: only accepted pending
Disable at the allocation tick blocks grants; it remains a scheduled command,
Enable has no lookahead, and no-gate behavior is unchanged. Native collision tests
must establish this invariant rather than assuming priorities are sufficient.
Objection: checkpoint loss. Response: fail closed immediately; no API acceptance
until the V2/consumer join. Objection: scope creep into staff policy. Response:
engine exposes one resource flag only; domain eligibility stays in CareOps.
Objection: event-budget rollback leaks flags/index. Response: include gate state
in the existing staged transaction and exercise exact-state rejection oracles.

## Bounded implementation order

1. Owner review this ADR and freeze exact API/error semantics; record downstream
   dependency handoff. Accepted CareOps space fixtures remain a dispatch gate.
2. Runtime packet: flow.rs, flow/checkpoint.rs and one dedicated focused native
   test path, plus result/logs (five paths). Include V1 rejection tests in that
   same test file. No V2 wire editing during runtime implementation.
3. Checkpoint V2 packet after runtime types freeze; no concurrent shared-type edits.
4. Calibration consumer packet and joined DES/core/ABM/calibration qualification.
5. Commit qualified Kairos changes, update parent pin separately, then integrate
   CareOps single-runtime behavior with all accepted E1 and E2 hand oracles.

Planned tests are not executed evidence. Full just ci, hosted/platform, binding,
performance and release acceptance remain separate unless actually run and passed.
