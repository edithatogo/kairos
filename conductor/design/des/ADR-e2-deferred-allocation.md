# E2 deferred allocation and complete checkpoint state

Status: coordinator contract independently reviewed by GPT-6.1-sol. No
implementation packet is authorized until separately bound and reviewed. Source base a27713ec5c8e903ecb8a9730d0cb4f105e8a7b83;
Rust/Cargo 1.99.0 only. Track03 owns Flow runtime/checkpoints; Track22 owns
calibration consumers; CareOps owns ED policy and the closed admission protocol.

## Requirement and decision

The accepted E1 triage contract requires all same-tick arrivals and completions,
including next-stage readiness, to be observed before selecting waiting work.
E2 requires native Suspend and composed simultaneous-event/resume cases. Existing
Flow eagerly allocates inside completion transactions. A parent observing the
completed dispatch cannot insert newly ready work before that allocation.
Predeclaring a contingent completion time is not a general solution.

Preserve unrestricted admitted E1/E2 composition. Add opt-in persistent deferred
allocation, not an uncheckpointable open phase. Default resources retain their
exact eager behavior. Calendar gates remain calendar gates. No second scheduler,
parent allocator, fake Suspend, tick delay or hidden composition prohibition.

Derived composed oracle (new E2.3 witness, not an existing E2.1 fixture ID): low
work10 starts1; urgent work2 ready4 suspends low after useful3, remaining7. Urgent
ends6. Another resource completes at6 and causally creates higher-priority B of
work1 for the first resource. Required intervals: low[1,4), urgent[4,6), B[6,7),
low[7,14). No zero-length low resume/preempt pair at6. Test both completion event
insertion orders, fixed-readiness control, and actual callback-created readiness.

## Proposed public native API

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowAllocationMode { Eager, Deferred }

impl FlowRuntime {
    pub fn create_resource_with_allocation_mode(
        &mut self, total: u32, mode: FlowAllocationMode,
    ) -> Result<ResourceId, FlowError>;
    pub fn resource_allocation_mode(
        &self, resource: ResourceId,
    ) -> Result<FlowAllocationMode, FlowError>;
    pub fn schedule_allocation_flush(
        &mut self, resource: ResourceId, at: SimTime, scheduler_priority: i32,
    ) -> Result<EventId, ResourceAllocationAdmissionError>;
}
```

ResourceAllocationAdmissionError has Flow(FlowError), NotDeferred and
DuplicateFlush variants; implement the same Clone/Copy/Debug/Eq/PartialEq,
Display/Error/source and From<FlowError> conventions as ResourceGrantAdmissionError.
No FrontierReached variant: current-tick flush admission is allowed. Freeze
validation precedence: live resource identity, Deferred mode, time, pending
Remove, duplicate pending boundary, normal scheduling/counter failures. Existing
Remove rejects a pending flush using its existing InvalidState error; no new
FlowError variant or old error wire tag is introduced.

Mode is immutable for a resource lifetime. Existing create_resource(total)
creates Eager. No live mode setter or policy mutation is included. Remove/recreate
gets a new generational identity and no inherited deferred state.

Normal events on Deferred resources still settle completions, deadlines,
release/cancel, capacity and request/progress transitions transactionally. They
skip only new grants, preemption selection and suspended-work resumption. This
includes Submit, Release, Capacity and GrantEnabled(true). Actual native queues,
leases, progress and accounting remain authoritative.

AllocationFlush(ResourceId) is a real scheduled Flow command. It is valid only
for a live Deferred resource, at >= now, including now after other same-tick
committed events. Unlike a calendar lookahead boundary it does not prohibit an
already-started tick. It settles due boundaries using the existing planner and
runs the existing allocation/preemption/resume algorithm for its target resource.
Other Deferred resources remain skipped even if affected by the same transaction.
It does not bypass a disabled flag or pending same-tick Disable. Disabled flush
is a valid no-grant dispatch. It never performs policy selection or changes mode.

At most one pending flush per (resource,tick); reject duplicates before mutation.
After that command commits, a later deliberate flush at the same tick may be
admitted. Reject Remove while a flush is pending, and flush admission while a
Remove is pending. Preserve existing gate/remove rules. Stale or removed targets
fail visibly. Validate identity, mode, time, duplicate, removal, all counters and
budgets before changing commands, scheduler or indices. Rejected admission must
match an untouched control on valid retry, including event IDs and counters.

Flush planning and commit use normal transition budgets, record order, callback
receipts, completion tokens and precommit halt semantics. No direct resource
mutation, synthetic event ID, separate allocator or skipped cost accounting.
A halt before flush commit retains pending command/index/resource/work/clock/ID
state except the existing budget-halt metadata, with exact control parity.
Admission checks scheduling/counter bounds; dispatch checks transition cost.
Scheduling a valid flush may succeed and subsequently halt at dispatch budget.
A checkpoint after earlier drain events retains their real committed state.

## Parent admission protocol and limits

The engine cannot infer whether external future ingress is complete. The parent
owns that obligation and an explicit set of resources needing allocation.
All pathway, terminal, transit and readiness events share this Flow scheduler.
Use private adapter priorities: fixed logical readiness ingress -1 (domain
readiness events, NOT early native Submit); completion/domain/notification and
completion-created readiness 0; calendar Enable 1; canonical native Submit
admissions after closure 2; allocation flush i32::MAX. No user priority may enter this scheduler phase field.
Request policy priority remains separately mapped from urgency using !urgency,
with readiness/ASCII ties encoded by canonical admission order. Merge fixed and
completion-created readiness before admitting either cohort; earlier knowledge
of a fixed request must not give it an earlier native tie sequence. Add a tie
control: fixed Z and completion-created A both ready6/equal urgency; A wins in
both completion insertion orders. Previously suspended native requests retain
original native admission/ready provenance; the parent must prove that this
agrees with its domain tie order. A handover is a new segment queued at its
handover tick, preserving logical sample lineage separately. Do not substitute
rank-based priority that would let equal-urgency work preempt, or assume native
sequence is automatically the domain ASCII order. The eventual parent packet
must demonstrate its complete admission-provenance invariant before dispatch.

Drain causally generated same-tick readiness through the scheduler before flush.
The ED work fixtures require positive service work, so granting it cannot create
an immediate same-tick completion that invalidates another resource's admission
closure. Zero-duration transit/domain continuations must reach a bounded fixed
point before allocation. Generic native zero-duration work retains normal native
semantics; this API does not promise a universal multi-resource atomic tick or
closed external ingress. A future domain requiring that stronger guarantee needs
its own demonstrated contract. Flush order is canonical by domain resource ID.

The parent must persist its logical pending/selection state with the composite
owner image when checkpointing it. A native checkpoint alone is not proof of
parent ED recovery. Unselected logical work may remain parent policy state;
active and suspended work, claims and progress must remain native.

## Versioned persistence; never extend accepted V1/V2 silently

Introduce FlowCheckpointV3 and FlowCommandV3. V3 carries the complete V2 image
plus canonical allocation_modes: Vec<(ResourceId, FlowAllocationMode)> covering
every live resource exactly once. FlowCommandV3 retains every V2 variant and adds
AllocationFlush(ResourceId). Borrowed/image routing extends losslessly to V3.
Common validation and restore consume complete state, never stripped V1/V2 DTOs.

V1 and V2 mean all resources Eager, no pending flush. Explicit V1/V2 capture must
fail closed before codecs if Deferred state or pending flush exists. Routed
capture selects V3 exactly when required, otherwise preserves accepted V1/V2
selection, errors and bytes. V1/V2 readers reject V3; known-magic router never
uses error fallback. Old software cannot read the new inner image; outer C2
composite envelope can remain V1 if its existing sections/bounds suffice.

Proposed V3 wire: magic `KFLOWV3\0`, u16 wire schema1, u32 image version3. Retain
V2 field order and command tags0..12, add tag13 AllocationFlush(ResourceId), then
append mode rows after V2 flags/frontier. Mode tags0=Eager,1=Deferred. Native DTO
and exact exported API signatures must be independently reviewed before the
native packet; no wire change is authorized before that join.

Persist mode rows and pending flush command/event joins. Derive pending-flush
index on restore; validate live capture cache, unique resource/tick membership,
mode/target identity, pending Remove conflicts, scheduler kind/time/entity and
complete command cardinality before codec calls. Flush scheduler entity is None.
Same-tick admitted flush after an already-started frontier is legal persistence.

Bound each mode row set by max_resources; flush count by max_commands. Include
flushes in aggregate pending operations with release+despawn+gates. Count mode
rows in aggregate records and bytes with checked arithmetic. Full nonallocating
V3 scanner must validate the entire payload before owning decode; encode must
measure full budgets before allocating canonical validators. Preserve all V1/V2
wire goldens and complete portable-envelope goldens where no Deferred mode exists.

## Bounded delivery and acceptance

1. Review this owner contract and exact APIs. Runtime leaf owns flow.rs, required
   fail-closed checkpoint guards and focused tests. Any size exception is explicit.
2. Native V3 capture/restore, then independent V3 wire scan/encode/decode, with
   complete-state borrowed consumer updates and clean joins. Parallelize only
   disjoint frozen consumer seams. Portable V2 packet already running at its
   accepted base remains untouched; future changes bind its accepted result.
3. Qualify combined native/core/ABM/calibration on Rust1.99, then parent pin under
   separate claim, parent single-Flow adapter, 88 fixture bindings and E2.3 mixed
   witness. Existing phase/release/clinical acceptance remains separate.

Required native oracles: eager trace/byte parity; deferred Submit/completion/
release/capacity/enable never grant without flush; flush urgency/tie/native
Suspend behavior; disabled/pending-disable flush suppression; both derived mixed
completion orders; no gratuitous resume; conservation; exact H grants; pending
flush duplicate/remove/time/mode/budget rejection and retry parity; generation
reuse; precommit budget halt; interior drain and preflush checkpoint+fresh-process
continuation parity; malformed mode/command/index joins rejected before codecs
with valid positive controls. No checkpoint-loss window may be accepted as a
substitute for these requirements.
