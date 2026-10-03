# Q3 experimental timed work and interruption contract

Coordinator contract after independent Q0 oracle/Track03 source review, 2026-10-03.
Implementations/phase acceptance still require actual tests and owner CI.

## Additive public API

`PreemptionStrategy::{Suspend,Abort,Restart}`; `WorkState::{Pending,Active,
Suspended,Completed,Aborted,Cancelled}`. `WorkProgress` exposes original_duration,
useful_elapsed, remaining, cumulative_busy (SimDuration), attempt_revision and
execution_revision (checkedu64), state, segment_started_at and completion_at
(Option<SimTime>). `FlowRuntime::work_progress(WorkId)` returns checkedinspection;
activeinspection may derivecurrentsegment effort withoutmutatingstored state.

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

`WorkHandlers<C>` holds `on_resume:Option<fn(&mut C,&WorkProgress)>` and
`on_restart:Option<fn(&mut C,&WorkProgress)>`;
`register_work_handlers<C:'static>(registration:&str,handlers:WorkHandlers<C>)`
checks contextTypeId, rejects duplicate/mismatchedregistration, and binds static
Rustfunction pointers. Callbacks run aftercommitted transitions, canonically in
transition order, exactly once; they receive context/progress and no runtime or
command ingress. No scheduling/ABM/generalcompletion-hook API untilQ4. Trusted
factories/callbacks/destructors are not panic/side-effect isolated. Arithmetic and
invalidcommand errors remain transactionally checked before ECS changes/hooks.
Restartfactory refresh happens before on_restart; Suspend retains ownedcontext.

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

Ordered queuefront wins; only a can_preempt queuefront may evict whenfull. Victim
must be timed/preemptible and have strictly worsepriority; amongeligible holders,
choose worstpriority, latest originaladmissionsequence, largestRequestId. One
unit replacement evicts oneholder. Suspend requeues preserving usefulwork,
remainingduration, originaladmissionsequence and context. Restart requeues with
originalduration/usefulelapsed0, preserved cumulativewastedeffort, checkednewattempt
and initialcontextreset onnextstart. Abort terminatesforever. All oldleases stale.
Firstgrant deadline stayscleared through interruption. Suspended cancellation is
terminal; waiting/suspended time adds no busy effort. Completion/release/cancel/
preemption accounts currentsegment once; original/cumulative efforts use checked
integer ticks. Ownercleanup covers progress and restarttemplate/context.

Oracles: low10at0/urgent2at3 => urgent5; lowSuspend12,Restart15,Abort3.
Independentlow10at0/urgent3at4 => urgent7; low13/17/Abort4. Staleoriginalcompletion10
is no-op. Testnested/multiplevictims/ties/independentflags/nonpreemptibleholders,
cancellationwhilesuspended, zero remaining/completionattick, zero duration and
arithmetic/revision/scheduleoverflow withoutpartialeviction. No new RNGdraws.
Persistent same-tick notification budget and full publicdomain hooks remainQ4;
this phase does not claim those or portablecontinuation serialization.
