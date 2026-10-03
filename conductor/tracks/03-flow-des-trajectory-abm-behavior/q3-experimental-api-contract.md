# Q3 experimental timed work and interruption contract

Coordinator contract after independent Q0 oracle/Track03 source review, 2026-10-03.
Implementations/phase acceptance still require actual tests and owner CI.

## Additive public API

`PreemptionStrategy::{Suspend,Abort,Restart}`; `WorkState::{Pending,Active,
Suspended,Completed,Aborted,Cancelled,Released}`. `WorkProgress` exposes original_duration,
useful_elapsed, remaining, cumulative_busy (SimDuration), attempt_revision and
execution_revision (checkedu64), state, segment_started_at and completion_at
(Option<SimTime>). `FlowRuntime::work_progress(WorkId)` returns checkedinspection;
active inspection derives current segment effort without mutating stored state.
Repeated inspection never checkpoints or double-counts effort; commit checkpoints
a segment exactly once before clearing its start timestamp.

Existing `for_work` and `submit_work` remain untimed contextassociation. New
`AcquireBuilder::timed_work(WorkId)` explicitly opts into timedcompletion;
`can_preempt(bool)` independently permitsincomingeviction, defaultfalse;
`preemptible(PreemptionStrategy)` enables timedholderinterruption, defaultdisabled.
Manual/untimed preemptible holders reject InvalidWork before request/events/work
association. Restart requires a registered owned initial context factory.

`create_restartable_work<T:'static,C:'static>(owner,duration,registration,
initial_template:T,make_context:fn(&T)->C)` owns its immutable initialtemplate and
factory. Factory must return logically fresh initialcontext, without new service
duration draws; original sampled duration stays immutable. This is a trusted
Rust-domain contract, not a claim that arbitrary shared/interior-mutable Clone
restores state. No public factory template mutation or portablecodec.
Ordinary `create_work` supports Suspend/Abort and nonpreemptible timedwork.

`WorkHandlers<C>` holds optional `on_resume`, `on_restart`, `on_abort`, and
`on_cancel` function pointers of type `fn(&mut C,&WorkProgress)`.
`register_work_handlers<C: 'static>(registration: &str, handlers: WorkHandlers<C>)`
checks context TypeId and rejects duplicate/mismatched registration. Registration
must occur before creating work using that registration. An absent registration
or absent handler is valid and creates no callback token. These are trusted Rust
callbacks, not serialized or dynamically loaded code.

Callbacks run only as later dispatched internal notification events (reserved
kind 4003), scheduler priority 0, in the committed transition order. Each token
contains originating event/ordinal and an owned progress snapshot. Delivery does
not emit another lifecycle row. Resume/restart use their corresponding handlers;
Abort uses on_abort exactly once; explicit cancellation uses on_cancel exactly
once, including cancellation while suspended. Token consumption prevents duplicate
delivery. Removed owner/context invalidates pending delivery as an observable
no-op. The callback receives context/progress, with no runtime/command ingress in
this Q3 API; Q4 adds checked command submission and general hooks. No callback
runs inside arbitration. Restart factory refresh happens before committing its
Restarted transition, then the later on_restart sees the fresh context.

Factories/callbacks/destructors are trusted and not panic/side-effect isolated.
Arithmetic, command validation, notification/completion token budgets and ordinal
limits are preflighted before ECS changes or factory operations. No broad rollback
of arbitrary user-code effects is promised.

## Canonical interruption and timing

Add RequestState::{Suspended,Completed,Aborted} and LifecycleTransition labels
Queued/Granted/Released/Cancelled/TimedOut/Preempted/Resumed/Restarted/Completed/
Aborted, retaining existing LifecycleRecord fields plus transition discriminator.
A resumed/restarted allocation emits Resumed/Restarted, never another Granted.

Everycommand-target resource first expiresdue waitingdeadlines and completesdue
active timedwork before explicitoperation/arbitration. Completion order is
(completion_at,RequestId), deadline order preserves Q2. Reservecompletionkind4001;
logical revision/lease/timestamp invalidates oldtokens as visible stale no-ops.
CompletionatT wins over evictionatT in both token insertion orders. Timedgrant
schedules checked now+remaining; preflight all replacementleases/revisions/effort/
newtoken budgets and ordinal bounds before ECSwrites or context/hook operations.
Zero duration grants then completes same tick without duplicate terminalrows.

Free units go to the first waiting claim in PriorityKey order. When full, scan
waiting claims in that order for an eligible can_preempt claim and victim pair;
a higher-ranked non-preempting waiter does not prevent a later eligible claim
from replacing a holder. Re-evaluate after each atomic replacement. Frozen oracle:
holder priority9; queued A priority1/cannot preempt; queued B priority2/can preempt.
B evicts the eligible holder while A stays queued, then A wins the next free unit.
This implements the parent spec's ordered consideration without an extra head-of-
line restriction. Victim
must be timed/preemptible and have strictly worsepriority; amongeligible holders,
choose worstpriority, latest originaladmissionsequence, largestRequestId. One
unit replacement evicts oneholder. Suspend requeues preserving usefulwork,
remainingduration, originaladmissionsequence and context. Restart requeues with
originalduration/usefulelapsed0, preserved cumulativewastedeffort, checkednewattempt
and initial context reset on next start. attempt_revision increments at Restart
interruption, not again on allocation; execution_revision increments on every
allocation. Suspend retains attempt_revision. Both counters fail closed. Abort terminatesforever. All oldleases stale.
Firstgrant deadline stayscleared through interruption. Suspended cancellation is
terminal; waiting/suspended time adds no busy effort. Completion/release/cancel/
preemption accounts current segment once. Explicit timed release sets request
Released and work Released, abandons remaining work, clears the segment/completion,
and emits only Released (no completion or cancellation callback); original/cumulative efforts use checked
integer ticks. Ownercleanup covers progress and restarttemplate/context.

Oracles: low10at0/urgent2at3 => urgent5; lowSuspend12,Restart15,Abort3.
Independentlow10at0/urgent3at4 => urgent7; low13/17/Abort4. Staleoriginalcompletion10
is no-op. Testnested/multiplevictims/ties/independentflags/nonpreemptibleholders,
cancellationwhilesuspended, zero remaining/completionattick, zero duration and
arithmetic/revision/scheduleoverflow withoutpartialeviction. No new RNGdraws.
Persistent same-tick notification budget and full publicdomain hooks remainQ4;
this phase does not claim those or portablecontinuation serialization.
