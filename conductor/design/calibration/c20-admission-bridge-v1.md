# C2.0 actual Flow admission bridge v1

Status: reviewed architecture; exact document awaits final independent readback.
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
actual timed acquire. Before retry read WorkSpec.request: existing request is
accepted only if every ResourceRequest field matches stored intent; mismatches
reject ConflictingSubmission. Scheduler priority is not stored on ResourceRequest:
matching its value requires retained actual admission EventId/preview evidence,
not an invented getter. The bridge must not assert full idempotency from incomplete
metadata. First submission stores RequestId and its admission evidence. Failed
submission returns Bound; no second successful submit can be inferred from fields
that are absent. External caller mutation of bound work is rejected, not reconciled
by guessing. Nonzero Micro submission follows accepted arrival receipts.

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
