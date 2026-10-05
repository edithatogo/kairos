# C2.0 actual Flow admission bridge v1

Status: test-interface contract accepted after independent owner readback and
coordinator resolution of the explicit-start correction; runtime remains absent.
This freezes test interfaces, not runtime acceptance. Track03 owns DES fidelity
and transaction hooks; Track21 owns optional calibration production adapter;
Track01 owns purpose keys; Track22 owns complete checkpoint restoration.
Provider contract: c20-work-provider-v1.md. No DES -> calibration dependency.

## Experimental production API gate

DES exports the reviewed fidelity boundary only after ADR, API form, conformance,
compatibility and objection review. Experimental development does not require a
stable release baseline; release remains separate. Calibration's optional `flow`
feature uses locked optional DES/ABM dependencies in production, never a test
path import as its only bridge. Default DES1.76 and calibration1.88 remain intact.

## Frozen policy permit

```rust
pub struct FidelityAdmissionPermit<'a> { /* private, non-Clone */ }
impl FidelityAdapter {
    pub fn prepare_admission<'a>(
        &'a mut self, flow: &FlowRuntime, owner: EntityId, subsystem: &str,
    ) -> Result<FidelityAdmissionPermit<'a>, FidelityError>;
}
impl<'a> FidelityAdmissionPermit<'a> {
    pub fn decision(&self) -> FidelityDecision;
    pub fn bind(self, flow: &FlowRuntime, work: WorkId, expected: SimDuration)
        -> Result<FidelityDecision, (Self, FidelityError)>;
}
```

Permit borrows the adapter mutably through actual creation/binding so staging or
applying policy cannot race the frozen decision. Preparation reads actual actor
identity and runtime lineage before resolving scope; Track03 adds a crate-private
Flow actor-validation helper rather than public world mutation. Invalid actor,
foreign runtime, duplicate binding, non-Pending work, different owner or sampled
original duration -> InvalidWork (duplicate -> DuplicateAdmission). Successful
binding inserts the frozen decision once. Failed checks change neither bindings
nor Flow and return permit for retry. Adapter lineage binds on first successful
binding, not a failed preparation. Permit decision/runtime/owner fields cannot be
forged, cloned, serialized or included in seed/event/checkpoint bytes. Existing
admit remains compatible and retains its original tests.

## Exact private calibration adapter states

Four non-Clone production types use actual engine handles, not simulated counters:

```rust
pub(crate) struct WorkPreparationInput<T: Clone, C: 'static> { /* private */ }
pub(crate) struct PreparedIntrinsicWork<'a,T: Clone,C:'static> { /* private */ }
pub(crate) struct CreatedIntrinsicWork<'a,T: Clone,C:'static> { /* private */ }
pub(crate) struct BoundIntrinsicWork<T: Clone,C:'static> { /* private */ }
pub(crate) struct SubmittedIntrinsicWork<T: Clone,C:'static> { /* private */ }
pub(crate) struct AcquireIntent { /* exact owned fields below */ }
pub(crate) enum BridgeError {
    Fidelity(FidelityError), Duration(WorkDurationError), Flow(FlowError),
    Transit(TransitError), ConflictingSubmission, InvalidDispatch,
}
pub(crate) struct PrepareFailure<T:Clone,C:'static> {
    pub(crate) input: WorkPreparationInput<T,C>, pub(crate) error: BridgeError,
}
pub(crate) struct CreateFailure<'a,T:Clone,C:'static> {
    pub(crate) prepared: PreparedIntrinsicWork<'a,T,C>, pub(crate) error: BridgeError,
}
pub(crate) struct BindFailure<'a,T:Clone,C:'static> {
    pub(crate) created: CreatedIntrinsicWork<'a,T,C>, pub(crate) error: BridgeError,
}
pub(crate) struct SubmitFailure<T:Clone,C:'static> {
    pub(crate) bound: BoundIntrinsicWork<T,C>, pub(crate) error: BridgeError,
}
impl<T:Clone,C:'static> WorkPreparationInput<T,C> {
    pub(crate) fn prepare<'a>(self, flow:&FlowRuntime,
        adapter:&'a mut FidelityAdapter, provider:&IntrinsicWorkProvider)
        -> Result<PreparedIntrinsicWork<'a,T,C>,PrepareFailure<T,C>>;
}
impl<'a,T:Clone,C:'static> PreparedIntrinsicWork<'a,T,C> {
    pub(crate) fn create(self, flow:&mut FlowRuntime)
        -> Result<CreatedIntrinsicWork<'a,T,C>,CreateFailure<'a,T,C>>;
}
impl<'a,T:Clone,C:'static> CreatedIntrinsicWork<'a,T,C> {
    pub(crate) fn work(&self) -> WorkId;
    pub(crate) fn bind(self, flow:&FlowRuntime)
        -> Result<BoundIntrinsicWork<T,C>,BindFailure<'a,T,C>>;
}
impl<T:Clone,C:'static> BoundIntrinsicWork<T,C> {
    pub(crate) fn work(&self) -> WorkId;
    pub(crate) fn submit(self, flow:&mut FlowRuntime)
        -> Result<SubmittedIntrinsicWork<T,C>,SubmitFailure<T,C>>;
}
impl<T:Clone,C:'static> SubmittedIntrinsicWork<T,C> {
    pub(crate) fn work(&self) -> WorkId;
    pub(crate) fn request(&self) -> RequestId;
    pub(crate) fn sampled_duration(&self) -> SimDuration;
}
```

Input owns owner/subsystem/registration/stratum strings, expected Service key,
one advancing Service stream, immutable template T, pure `fn(&T)->C` factory,
AcquireIntent and explicit transit request (Zero or graph/profile/O-D request).
Prepare obtains permit, validates intent/identity/route for the frozen mode, then
samples last. There is no fallible work after a successful sample before returning
Prepared. On preparation failure return unchanged input; Macro never routes/draws
transit and Zero Micro creates no route/event. Prepared retains sample+stream.

Create uses real `create_restartable_work` with template.clone() and the pure
factory. Both existing create functions consume arguments even on error; retaining
the original template is mandatory. A factory may rebuild transient context but
must not draw RNG or cause external effects. Failure returns Prepared intact;
success stores the actual WorkId. Bind compares actual original_duration, owner,
Pending state and runtime lineage and consumes frozen permit; failure returns the
Created handle, never an earlier state that would recreate work. Bound retains
sample/service continuation and exact intent. No stream/sample Clone or resampling
on retry, policy change, Suspend or Restart. Template cloning is not RNG cloning.

AcquireIntent owns resource, owner, at, priority_level, optional deadline,
scheduler_priority, can_preempt, optional preemptible strategy; work is inserted
from the actual bound handle and timed is always true. Macro and Zero Micro call
actual timed acquire. A successful direct submit immediately returns Submitted.
Bound retry accepts only WorkSpec.request=None. Some(request) without this bridge's
own accepted receipt -> ConflictingSubmission, even if exposed fields happen to
match. ResourceRequest omits scheduler priority; no invented getter or generic
external-request recovery is permitted. Accepted callback receipt proves the
bridge's exact stored emitted command/ticket/event/request and is reconciled with
actual WorkSpec.request/ResourceRequest. Full intent comparisons use the retained
command; lost evidence remains unrecoverable. A failed stage retains original
sample, stream, template and current actual work handle.

## Constructible adapter and asynchronous Micro interfaces

```rust
pub(crate) struct PreparationIdentity {
    pub(crate) owner:EntityId, pub(crate) subsystem:String,
    pub(crate) registration:String, pub(crate) stratum:String,
}
pub(crate) struct AcquireIntent {
    pub(crate) resource:ResourceId, pub(crate) owner:EntityId,
    pub(crate) at:SimTime, pub(crate) priority_level:i32,
    pub(crate) deadline:Option<SimTime>, pub(crate) scheduler_priority:i32,
    pub(crate) can_preempt:bool,
    pub(crate) preemptible:Option<PreemptionStrategy>,
}
pub(crate) enum TransitRequest {
    Zero,
    Route {
        graph:std::sync::Arc<TransitGraphV1>, origin:NodeId,destination:NodeId,
        profile:MovementProfile,ticks_per_second:u64,
        carrier_actor:EntityId,carrier_registration:String,kind:EventKind,
    },
}
impl<T:Clone,C:'static> WorkPreparationInput<T,C> {
    pub(crate) fn new(identity:PreparationIdentity,
        service:CalibrationStream,expected:CalibrationStreamKey,
        template:T,make_context:fn(&T)->C,intent:AcquireIntent,
        transit:TransitRequest)->Self;
}
impl<'a,T:Clone,C:'static> PreparedIntrinsicWork<'a,T,C> {
    pub(crate) fn sampled_duration(&self)->SimDuration;
    pub(crate) fn service_draw_position(&self)->u64;
}
impl<T:Clone,C:'static> BoundIntrinsicWork<T,C> {
    pub(crate) fn sampled_duration(&self)->SimDuration;
    pub(crate) fn service_draw_position(&self)->u64;
    pub(crate) fn start_transit(&mut self,flow:&mut FlowRuntime)
        ->Result<EventId,BridgeError>;
    pub(crate) fn observe_transit_dispatch(&mut self,flow:&FlowRuntime,
        dispatch:&FlowDispatch)->Result<TransitObservation,BridgeError>;
    pub(crate) fn retry_transit(&mut self,flow:&mut FlowRuntime,
        rejected:&FlowDispatch)->Result<EventId,BridgeError>;
    pub(crate) fn finish_transit(self,flow:&FlowRuntime)
        ->Result<SubmittedIntrinsicWork<T,C>,SubmitFailure<T,C>>;
}
pub(crate) enum TransitObservation {
    Progress,Paused,Resumed,IgnoredStale,Rejected,Arrived,
}
```

Constructors merely retain input; prepare validates canonical identity strings,
owner equality, future at/deadline and resource existence before service sampling.
Macro ignores Route input; explicit Zero Micro bypasses transit. Bound.submit is
only direct Macro/Zero; nonzero Micro returns a Flow InvalidState error with Bound
retained. Nonzero Micro uses start/observe/retry/finish. Input intent.at is the
planned route start; actual arrival replaces the timed command's at field with
FlowWorldView.now. Original fields and this adjustment rule remain retained.

Start creates/reuses Ready context and schedules its initial ordinary domain event
at AcquireIntent.at; it never starts elapsed movement synchronously. Start retains
a cloneable route/carrier context on carrier-creation failure;
once created its actual carrier WorkId remains stored and is reused on scheduling
retry, never recreated. Existing unrelated/nonterminal carrier -> conflict, no
silent overwrite. Reuse is explicit through the registered transit adapter only
when its previous movement and resource claim are terminal. Hook registration is
setup work before task creation and never silently performed by a sampling call.

TransitContext stores DES FlowAcquireCommand, not calibration AcquireIntent.
The bridge constructs it after real task creation with work=Some(actual task),
timed=true and actual task owner. Carrier may be a distinct selected staff actor;
model adapter validates assignment rather than equating unrelated actor IDs.
Bound tracks its current progress EventId and accepted command tickets plus its
own scheduled control EventIds. Dispatch observation validates runtime first and
matches actual source/receipt. finish requires Arrived, own accepted arrival
receipt and linked request. A new request inferred only from mutable metadata
cannot produce Submitted. Retry of a rejected attempt is explicit and one-shot;
control/source event consumption remains actual scheduler behavior.

## Required test oracles and remaining gates

- Permit freezes precedence before sample/create; policy mutation while borrowed
  is rejected by Rust ownership. Foreign-runtime bind fails before work reads.
- Actual sample, WorkSpec and context/template survive create/bind/submit errors;
  retry never creates another task or spends another Service draw.
- Real Suspend resumes remaining work; Restart uses original sample/template and
  leaves separately owned transit progress intact.
- Compare intended versus actual request and admission evidence, including external
  conflicting submission and duplicate retries. No ghost request is manufactured.
- Provider snapshots do not restore FlowRuntime. Track22 complete runner bridge
  must preserve scheduler, resources, work, policy, routes and all streams before
  full continuation acceptance. Unknown versions/graph/profile mismatch fail closed.

No full C2.0/C2.1 acceptance follows from this contract. Test-first fixtures and
final native exact-head gates remain required. Parallel C4/Track49 paths untouched.
