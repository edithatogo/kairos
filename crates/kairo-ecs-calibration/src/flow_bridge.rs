//! Private, experimental calibration-to-Flow admission bridge.
//!
//! The first runtime slice accepts Macro and explicit Zero-transit Micro.
//! Route execution, callback receipts and checkpoint restoration are separate
//! contracts. The states below keep the single Service stream and sampled
//! duration owned across every fallible create/bind/submit transition.

use crate::seed_map::{CalibrationStream, CalibrationStreamKey, SeedPurpose};
use crate::work_duration::{IntrinsicWorkProvider, SampledWorkDuration, WorkDurationError};
use kairo_ecs_des::fidelity::{
    FidelityAdapter, FidelityAdmissionPermit, FidelityDecision, FidelityError, FidelityMode,
};
use kairo_ecs_des::{
    FlowError, FlowRuntime, FlowRuntimeIdentity, PreemptionStrategy, RequestId, ResourceId, WorkId,
    WorkState,
};
use kairo_ecs_types::{EntityId, SimDuration, SimTime};
use std::marker::PhantomData;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TransitIntent {
    Zero,
    Route,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AcquireIntent {
    pub(crate) resource: ResourceId,
    pub(crate) owner: EntityId,
    pub(crate) at: SimTime,
    pub(crate) priority_level: i32,
    pub(crate) deadline: Option<SimTime>,
    pub(crate) scheduler_priority: i32,
    pub(crate) can_preempt: bool,
    pub(crate) preemptible: Option<PreemptionStrategy>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BridgeError {
    Fidelity(FidelityError),
    Duration(WorkDurationError),
    Flow(FlowError),
    ConflictingSubmission,
    InvalidDispatch,
}

pub(crate) struct WorkPreparationInput<T: Clone, C: 'static> {
    pub(crate) owner: EntityId,
    pub(crate) subsystem: String,
    pub(crate) stratum: String,
    pub(crate) expected_service_key: CalibrationStreamKey,
    pub(crate) service_stream: CalibrationStream,
    pub(crate) template: T,
    pub(crate) registration: String,
    pub(crate) make_context: fn(&T) -> C,
    pub(crate) acquire: AcquireIntent,
    pub(crate) transit: TransitIntent,
}

pub(crate) struct PrepareFailure<T: Clone, C: 'static> {
    pub(crate) input: WorkPreparationInput<T, C>,
    pub(crate) error: BridgeError,
}

pub(crate) struct PreparedIntrinsicWork<'a, T: Clone, C: 'static> {
    permit: FidelityAdmissionPermit<'a>,
    decision: FidelityDecision,
    expected_service_key: CalibrationStreamKey,
    service_stream: CalibrationStream,
    sample: SampledWorkDuration,
    template: T,
    registration: String,
    make_context: fn(&T) -> C,
    acquire: AcquireIntent,
}

pub(crate) struct CreateFailure<'a, T: Clone, C: 'static> {
    pub(crate) prepared: PreparedIntrinsicWork<'a, T, C>,
    pub(crate) error: BridgeError,
}

pub(crate) struct CreatedIntrinsicWork<'a, T: Clone, C: 'static> {
    permit: FidelityAdmissionPermit<'a>,
    decision: FidelityDecision,
    expected_service_key: CalibrationStreamKey,
    service_stream: CalibrationStream,
    sample: SampledWorkDuration,
    acquire: AcquireIntent,
    work: WorkId,
    _restart_types: PhantomData<fn() -> (T, C)>,
}

pub(crate) struct BindFailure<'a, T: Clone, C: 'static> {
    pub(crate) created: CreatedIntrinsicWork<'a, T, C>,
    pub(crate) error: BridgeError,
}

pub(crate) struct BoundIntrinsicWork<T: Clone, C: 'static> {
    decision: FidelityDecision,
    expected_service_key: CalibrationStreamKey,
    service_stream: CalibrationStream,
    sample: SampledWorkDuration,
    acquire: AcquireIntent,
    runtime: FlowRuntimeIdentity,
    work: WorkId,
    _restart_types: PhantomData<fn() -> (T, C)>,
}

pub(crate) struct SubmitFailure<T: Clone, C: 'static> {
    pub(crate) bound: BoundIntrinsicWork<T, C>,
    pub(crate) error: BridgeError,
}

pub(crate) struct SubmittedIntrinsicWork<T: Clone, C: 'static> {
    decision: FidelityDecision,
    expected_service_key: CalibrationStreamKey,
    service_stream: CalibrationStream,
    sample: SampledWorkDuration,
    acquire: AcquireIntent,
    work: WorkId,
    request: RequestId,
    _restart_types: PhantomData<fn() -> (T, C)>,
}

impl<T: Clone + 'static, C: 'static> WorkPreparationInput<T, C> {
    #[allow(clippy::result_large_err)]
    pub(crate) fn prepare<'a>(
        mut self,
        flow: &FlowRuntime,
        adapter: &'a mut FidelityAdapter,
        provider: &IntrinsicWorkProvider,
    ) -> Result<PreparedIntrinsicWork<'a, T, C>, PrepareFailure<T, C>> {
        let permit = match adapter.prepare_admission(flow, self.owner, &self.subsystem) {
            Ok(permit) => permit,
            Err(error) => {
                return Err(PrepareFailure {
                    input: self,
                    error: BridgeError::Fidelity(error),
                });
            }
        };
        let decision = permit.decision();

        let validation = if self.owner != self.acquire.owner
            || self.acquire.at < flow.now()
            || self
                .acquire
                .deadline
                .is_some_and(|deadline| deadline <= self.acquire.at)
        {
            Err(BridgeError::InvalidDispatch)
        } else if flow.resource(self.acquire.resource).is_err() {
            Err(BridgeError::Flow(FlowError::InvalidResource))
        } else if decision.mode == FidelityMode::Micro && self.transit == TransitIntent::Route {
            // C2.3 owns actual route dispatch. Reject before touching Service.
            Err(BridgeError::InvalidDispatch)
        } else if self.service_stream.purpose() != SeedPurpose::Service {
            Err(BridgeError::Duration(WorkDurationError::WrongPurpose))
        } else if self.service_stream.key() != self.expected_service_key {
            Err(BridgeError::Duration(WorkDurationError::IdentityMismatch))
        } else {
            Ok(())
        };
        if let Err(error) = validation {
            drop(permit);
            return Err(PrepareFailure { input: self, error });
        }

        // Sampling is the last fallible operation in prepare. The provider
        // commits the temporary stream only on success; all earlier failures
        // return this unchanged input, while success moves its sole stream on.
        let sample = match provider.sample(
            &self.stratum,
            &mut self.service_stream,
            &self.expected_service_key,
        ) {
            Ok(sample) => sample,
            Err(error) => {
                drop(permit);
                return Err(PrepareFailure {
                    input: self,
                    error: BridgeError::Duration(error),
                });
            }
        };

        Ok(PreparedIntrinsicWork {
            permit,
            decision,
            expected_service_key: self.expected_service_key,
            service_stream: self.service_stream,
            sample,
            template: self.template,
            registration: self.registration,
            make_context: self.make_context,
            acquire: self.acquire,
        })
    }
}

impl<'a, T: Clone + 'static, C: 'static> PreparedIntrinsicWork<'a, T, C> {
    pub(crate) fn decision(&self) -> FidelityDecision {
        self.decision
    }

    pub(crate) fn sampled_duration(&self) -> SimDuration {
        self.sample.duration()
    }

    pub(crate) fn draw_bounds(&self) -> (u64, u64) {
        (self.sample.draw_before(), self.sample.draw_after())
    }

    #[allow(clippy::result_large_err)]
    pub(crate) fn create(
        self,
        flow: &mut FlowRuntime,
    ) -> Result<CreatedIntrinsicWork<'a, T, C>, CreateFailure<'a, T, C>> {
        let Self {
            permit,
            decision,
            expected_service_key,
            service_stream,
            sample,
            template,
            registration,
            make_context,
            acquire,
        } = self;
        match flow.create_restartable_work(
            acquire.owner,
            sample.duration(),
            &registration,
            template.clone(),
            make_context,
        ) {
            Ok(work) => Ok(CreatedIntrinsicWork {
                permit,
                decision,
                expected_service_key,
                service_stream,
                sample,
                acquire,
                work,
                _restart_types: PhantomData,
            }),
            Err(error) => Err(CreateFailure {
                prepared: PreparedIntrinsicWork {
                    permit,
                    decision,
                    expected_service_key,
                    service_stream,
                    sample,
                    template,
                    registration,
                    make_context,
                    acquire,
                },
                error: BridgeError::Flow(error),
            }),
        }
    }
}

impl<'a, T: Clone + 'static, C: 'static> CreatedIntrinsicWork<'a, T, C> {
    pub(crate) fn work(&self) -> WorkId {
        self.work
    }

    pub(crate) fn decision(&self) -> FidelityDecision {
        self.decision
    }

    #[allow(clippy::result_large_err)]
    pub(crate) fn bind(
        self,
        flow: &FlowRuntime,
    ) -> Result<BoundIntrinsicWork<T, C>, BindFailure<'a, T, C>> {
        match self.permit.bind(flow, self.work, self.sample.duration()) {
            Ok(_) => Ok(BoundIntrinsicWork {
                decision: self.decision,
                expected_service_key: self.expected_service_key,
                service_stream: self.service_stream,
                sample: self.sample,
                acquire: self.acquire,
                runtime: flow.identity(),
                work: self.work,
                _restart_types: PhantomData,
            }),
            Err((permit, error)) => Err(BindFailure {
                created: CreatedIntrinsicWork { permit, ..self },
                error: BridgeError::Fidelity(error),
            }),
        }
    }
}

impl<T: Clone + 'static, C: 'static> BoundIntrinsicWork<T, C> {
    pub(crate) fn work(&self) -> WorkId {
        self.work
    }

    pub(crate) fn draw_position(&self) -> u64 {
        self.service_stream.draw_position()
    }

    pub(crate) fn decision(&self) -> FidelityDecision {
        self.decision
    }

    pub(crate) fn service_identity_matches(&self) -> bool {
        self.service_stream.key() == self.expected_service_key
    }

    pub(crate) fn acquire_intent(&self) -> &AcquireIntent {
        &self.acquire
    }

    #[allow(clippy::result_large_err)]
    pub(crate) fn submit(
        self,
        flow: &mut FlowRuntime,
    ) -> Result<SubmittedIntrinsicWork<T, C>, SubmitFailure<T, C>> {
        if self.runtime != flow.identity() {
            return Err(SubmitFailure {
                bound: self,
                error: BridgeError::InvalidDispatch,
            });
        }
        if !self.service_identity_matches() {
            return Err(SubmitFailure {
                bound: self,
                error: BridgeError::Duration(WorkDurationError::IdentityMismatch),
            });
        }
        let spec = match flow.work(self.work) {
            Ok(spec) => spec,
            Err(error) => {
                return Err(SubmitFailure {
                    bound: self,
                    error: BridgeError::Flow(error),
                });
            }
        };
        if spec.request.is_some() {
            return Err(SubmitFailure {
                bound: self,
                error: BridgeError::ConflictingSubmission,
            });
        }
        let progress = match flow.work_progress(self.work) {
            Ok(progress) => progress,
            Err(error) => {
                return Err(SubmitFailure {
                    bound: self,
                    error: BridgeError::Flow(error),
                });
            }
        };
        if spec.owner != self.acquire.owner
            || spec.original_duration != self.sample.duration()
            || progress.state != WorkState::Pending
        {
            return Err(SubmitFailure {
                bound: self,
                error: BridgeError::InvalidDispatch,
            });
        }

        let mut builder = flow
            .acquire(self.acquire.resource)
            .owner(self.acquire.owner)
            .at(self.acquire.at)
            .priority(self.acquire.priority_level)
            .scheduler_priority(self.acquire.scheduler_priority)
            .can_preempt(self.acquire.can_preempt)
            .timed_work(self.work);
        if let Some(deadline) = self.acquire.deadline {
            builder = builder.deadline(deadline);
        }
        if let Some(strategy) = self.acquire.preemptible {
            builder = builder.preemptible(strategy);
        }
        match builder.submit() {
            Ok(request) => {
                // Treat the runtime's ID as a candidate until both sides of
                // the authoritative work/request association agree.
                let request_matches = flow.request(request).is_ok_and(|saved| {
                    saved.resource == self.acquire.resource
                        && saved.owner == self.acquire.owner
                        && saved.work == Some(self.work)
                        && saved.timed
                        && saved.submitted_at == self.acquire.at
                        && saved.priority_level == self.acquire.priority_level
                        && saved.deadline == self.acquire.deadline
                        && saved.can_preempt == self.acquire.can_preempt
                        && saved.preemptible == self.acquire.preemptible
                });
                let work_matches = flow
                    .work(self.work)
                    .is_ok_and(|saved| saved.request == Some(request));
                if !request_matches || !work_matches {
                    // The request may already exist, so retain the bound
                    // state for diagnosis; its normal retry path will fail
                    // closed on the existing WorkSpec.request association.
                    return Err(SubmitFailure {
                        bound: self,
                        error: BridgeError::InvalidDispatch,
                    });
                }
                Ok(SubmittedIntrinsicWork {
                    decision: self.decision,
                    expected_service_key: self.expected_service_key,
                    service_stream: self.service_stream,
                    sample: self.sample,
                    acquire: self.acquire,
                    work: self.work,
                    request,
                    _restart_types: PhantomData,
                })
            }
            Err(error) => Err(SubmitFailure {
                bound: self,
                error: BridgeError::Flow(error),
            }),
        }
    }
}

impl<T: Clone + 'static, C: 'static> SubmittedIntrinsicWork<T, C> {
    pub(crate) fn work(&self) -> WorkId {
        self.work
    }

    pub(crate) fn request(&self) -> RequestId {
        self.request
    }

    pub(crate) fn sampled_duration(&self) -> SimDuration {
        self.sample.duration()
    }

    pub(crate) fn draw_position(&self) -> u64 {
        self.service_stream.draw_position()
    }

    pub(crate) fn decision(&self) -> FidelityDecision {
        self.decision
    }

    pub(crate) fn service_identity_matches(&self) -> bool {
        self.service_stream.key() == self.expected_service_key
    }

    pub(crate) fn acquire_intent(&self) -> &AcquireIntent {
        &self.acquire
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed_map::CalibrationSeedMap;
    use crate::work_duration::{IntrinsicDurationDistribution, INTRINSIC_WORK_PROVIDER_VERSION_V1};
    use kairo_ecs_des::fidelity::FidelityPolicy;
    use kairo_ecs_des::{RequestState, WorkHandlers};

    fn make_context(template: &u32) -> u32 {
        *template
    }

    fn provider() -> IntrinsicWorkProvider {
        IntrinsicWorkProvider::new(
            INTRINSIC_WORK_PROVIDER_VERSION_V1,
            vec![(
                "triage".to_owned(),
                IntrinsicDurationDistribution::weighted_ticks(vec![(10, 1), (20, 1)]).unwrap(),
            )],
        )
        .unwrap()
    }

    fn input(
        mode: FidelityMode,
        transit: TransitIntent,
        purpose: SeedPurpose,
        mismatch_key: bool,
    ) -> (
        WorkPreparationInput<u32, u32>,
        FlowRuntime,
        FidelityAdapter,
        ResourceId,
    ) {
        let mut flow = FlowRuntime::new();
        let owner = flow.spawn_actor().unwrap();
        let resource = flow.create_resource(1).unwrap();

        let mut seed_map = CalibrationSeedMap::new(1, "bridge-test", 19).unwrap();
        let key = seed_map
            .key_for("paired", 0, "case-a", "task-a", SeedPurpose::Service)
            .unwrap();
        let stream_task = if mismatch_key { "task-b" } else { "task-a" };
        let service_stream = seed_map
            .stream_for("paired", 0, "case-a", stream_task, purpose)
            .unwrap();
        let input = WorkPreparationInput {
            owner,
            subsystem: "assessment".to_owned(),
            stratum: "triage".to_owned(),
            expected_service_key: key,
            service_stream,
            template: 42,
            registration: "bridge.context".to_owned(),
            make_context,
            acquire: AcquireIntent {
                resource,
                owner,
                at: SimTime::from_ticks(0),
                priority_level: 3,
                deadline: None,
                scheduler_priority: 7,
                can_preempt: false,
                preemptible: None,
            },
            transit,
        };
        let adapter = FidelityAdapter::new(FidelityPolicy::new(1, Some(mode)).unwrap());
        (input, flow, adapter, resource)
    }

    fn must_prepare<'a>(
        result: Result<PreparedIntrinsicWork<'a, u32, u32>, PrepareFailure<u32, u32>>,
    ) -> PreparedIntrinsicWork<'a, u32, u32> {
        match result {
            Ok(value) => value,
            Err(failure) => panic!("prepare failed: {:?}", failure.error),
        }
    }

    fn must_create<'a>(
        result: Result<CreatedIntrinsicWork<'a, u32, u32>, CreateFailure<'a, u32, u32>>,
    ) -> CreatedIntrinsicWork<'a, u32, u32> {
        match result {
            Ok(value) => value,
            Err(failure) => panic!("create failed: {:?}", failure.error),
        }
    }

    fn must_bind<'a>(
        result: Result<BoundIntrinsicWork<u32, u32>, BindFailure<'a, u32, u32>>,
    ) -> BoundIntrinsicWork<u32, u32> {
        match result {
            Ok(value) => value,
            Err(failure) => panic!("bind failed: {:?}", failure.error),
        }
    }

    fn must_submit(
        result: Result<SubmittedIntrinsicWork<u32, u32>, SubmitFailure<u32, u32>>,
    ) -> SubmittedIntrinsicWork<u32, u32> {
        match result {
            Ok(value) => value,
            Err(failure) => panic!("submit failed: {:?}", failure.error),
        }
    }

    #[test]
    fn macro_and_zero_micro_share_service_draws_and_submit_actual_timed_work() {
        let (macro_input, mut macro_flow, mut macro_adapter, macro_resource) = input(
            FidelityMode::Macro,
            TransitIntent::Route,
            SeedPurpose::Service,
            false,
        );
        let macro_prepared =
            must_prepare(macro_input.prepare(&macro_flow, &mut macro_adapter, &provider()));
        assert_eq!(macro_prepared.decision().mode, FidelityMode::Macro);
        assert_eq!(macro_prepared.draw_bounds().0, 0);
        assert_eq!(macro_prepared.draw_bounds().1, 1);
        macro_flow
            .register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let macro_work = must_create(macro_prepared.create(&mut macro_flow));
        assert_eq!(macro_work.decision().mode, FidelityMode::Macro);
        assert_eq!(macro_flow.budget_snapshot().scheduler.scheduled_events, 0);
        let macro_bound = must_bind(macro_work.bind(&macro_flow));
        assert_eq!(macro_bound.decision().mode, FidelityMode::Macro);
        assert!(macro_bound.service_identity_matches());
        assert_eq!(macro_bound.acquire_intent().scheduler_priority, 7);
        assert_eq!(macro_flow.budget_snapshot().scheduler.scheduled_events, 0);
        let macro_submitted = must_submit(macro_bound.submit(&mut macro_flow));
        assert_eq!(macro_flow.budget_snapshot().scheduler.scheduled_events, 1);

        let (micro_input, mut micro_flow, mut micro_adapter, micro_resource) = input(
            FidelityMode::Micro,
            TransitIntent::Zero,
            SeedPurpose::Service,
            false,
        );
        let micro_prepared =
            must_prepare(micro_input.prepare(&micro_flow, &mut micro_adapter, &provider()));
        assert_eq!(micro_prepared.decision().mode, FidelityMode::Micro);
        assert_eq!(micro_prepared.draw_bounds(), (0, 1));
        micro_flow
            .register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let micro_work = must_create(micro_prepared.create(&mut micro_flow));
        assert_eq!(micro_work.decision().mode, FidelityMode::Micro);
        assert_eq!(micro_flow.budget_snapshot().scheduler.scheduled_events, 0);
        let micro_bound = must_bind(micro_work.bind(&micro_flow));
        assert_eq!(micro_bound.decision().mode, FidelityMode::Micro);
        assert_eq!(micro_flow.budget_snapshot().scheduler.scheduled_events, 0);
        let micro_submitted = must_submit(micro_bound.submit(&mut micro_flow));
        assert_eq!(micro_flow.budget_snapshot().scheduler.scheduled_events, 1);

        assert_eq!(
            macro_submitted.sampled_duration(),
            micro_submitted.sampled_duration()
        );
        assert_eq!(
            macro_submitted.draw_position(),
            micro_submitted.draw_position()
        );
        assert!(macro_submitted.service_identity_matches());
        assert_eq!(macro_submitted.decision().mode, FidelityMode::Macro);
        assert_eq!(macro_submitted.acquire_intent().scheduler_priority, 7);
        for (flow, resource, submitted) in [
            (&mut macro_flow, macro_resource, &macro_submitted),
            (&mut micro_flow, micro_resource, &micro_submitted),
        ] {
            assert_eq!(
                submitted.sampled_duration(),
                flow.work(submitted.work()).unwrap().original_duration
            );
            let spec = flow.work(submitted.work()).unwrap();
            assert_eq!(spec.request, Some(submitted.request()));
            let request = flow.request(submitted.request()).unwrap();
            assert_eq!(request.resource, resource);
            assert_eq!(request.owner, spec.owner);
            assert_eq!(request.work, Some(submitted.work()));
            assert!(request.timed);
            assert_eq!(request.priority_level, 3);
            assert_eq!(request.deadline, None);
            assert!(!request.can_preempt);
            assert_eq!(request.state, RequestState::Pending);
            assert!(flow.step().unwrap().is_some());
            assert_eq!(
                flow.request(submitted.request()).unwrap().state,
                RequestState::Active
            );
        }
    }

    #[test]
    fn micro_route_rejection_returns_unchanged_service_stream_before_sampling() {
        let (input, flow, mut adapter, _) = input(
            FidelityMode::Micro,
            TransitIntent::Route,
            SeedPurpose::Service,
            false,
        );
        let failure = match input.prepare(&flow, &mut adapter, &provider()) {
            Ok(_) => panic!("Micro Route should be rejected before sampling"),
            Err(failure) => failure,
        };
        assert_eq!(failure.error, BridgeError::InvalidDispatch);
        assert_eq!(failure.input.service_stream.draw_position(), 0);
        let mut retry_stream = failure.input.service_stream;
        let mut fresh = CalibrationSeedMap::new(1, "bridge-test", 19)
            .unwrap()
            .stream_for("paired", 0, "case-a", "task-a", SeedPurpose::Service)
            .unwrap();
        assert_eq!(retry_stream.next_u64(), fresh.next_u64());
    }

    #[test]
    fn wrong_service_purpose_and_identity_reject_without_advancing() {
        for (purpose, mismatch) in [(SeedPurpose::Transit, false), (SeedPurpose::Service, true)] {
            let (input, flow, mut adapter, _) =
                input(FidelityMode::Macro, TransitIntent::Zero, purpose, mismatch);
            let failure = match input.prepare(&flow, &mut adapter, &provider()) {
                Ok(_) => panic!("invalid Service stream should be rejected"),
                Err(failure) => failure,
            };
            assert_eq!(failure.input.service_stream.draw_position(), 0);
            assert!(matches!(failure.error, BridgeError::Duration(_)));
        }
    }

    #[test]
    fn create_failure_retains_template_sample_and_stream_for_retry() {
        let (input, mut flow, mut adapter, _) = input(
            FidelityMode::Macro,
            TransitIntent::Zero,
            SeedPurpose::Service,
            false,
        );
        let mut prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        let duration = prepared.sampled_duration();
        let draws = prepared.draw_bounds();
        let before = flow.budget_snapshot().scheduler;
        prepared.registration.clear();
        let failure = match prepared.create(&mut flow) {
            Ok(_) => panic!("invalid registration should fail creation"),
            Err(failure) => failure,
        };
        assert!(matches!(failure.error, BridgeError::Flow(_)));
        assert_eq!(failure.prepared.sampled_duration(), duration);
        assert_eq!(failure.prepared.draw_bounds(), draws);
        assert_eq!(failure.prepared.service_stream.draw_position(), draws.1);
        assert_eq!(flow.budget_snapshot().scheduler, before);
        let mut prepared = failure.prepared;
        prepared.registration = "bridge.context".to_owned();
        let created = must_create(prepared.create(&mut flow));
        assert_eq!(*flow.work_context::<u32>(created.work()).unwrap(), 42);
        assert_eq!(
            flow.work(created.work()).unwrap().original_duration,
            duration
        );
    }

    #[test]
    fn bind_failure_keeps_same_created_work_for_retry() {
        let (input, mut flow, mut adapter, _) = input(
            FidelityMode::Micro,
            TransitIntent::Zero,
            SeedPurpose::Service,
            false,
        );
        flow.register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        let created = must_create(prepared.create(&mut flow));
        let work = created.work();
        let foreign = FlowRuntime::new();
        let failure = match created.bind(&foreign) {
            Ok(_) => panic!("foreign Flow runtime must not bind work"),
            Err(failure) => failure,
        };
        assert_eq!(
            failure.error,
            BridgeError::Fidelity(FidelityError::InvalidWork)
        );
        assert_eq!(failure.created.work(), work);
        assert_eq!(flow.work(work).unwrap().request, None);
        let bound = must_bind(failure.created.bind(&flow));
        assert_eq!(bound.work(), work);
    }

    #[test]
    fn foreign_runtime_submit_failure_retains_bound_work_for_retry() {
        let (input, mut flow, mut adapter, _) = input(
            FidelityMode::Macro,
            TransitIntent::Zero,
            SeedPurpose::Service,
            false,
        );
        flow.register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        let created = must_create(prepared.create(&mut flow));
        let bound = must_bind(created.bind(&flow));
        let work = bound.work();
        let draw_position = bound.draw_position();
        let mut foreign = FlowRuntime::new();
        let foreign_events = foreign.budget_snapshot().scheduler.scheduled_events;
        let failure = match bound.submit(&mut foreign) {
            Ok(_) => panic!("foreign Flow runtime must not submit bound work"),
            Err(failure) => failure,
        };
        assert_eq!(failure.error, BridgeError::InvalidDispatch);
        assert_eq!(failure.bound.work(), work);
        assert_eq!(failure.bound.draw_position(), draw_position);
        assert_eq!(
            foreign.budget_snapshot().scheduler.scheduled_events,
            foreign_events
        );
        let submitted = must_submit(failure.bound.submit(&mut flow));
        assert_eq!(submitted.work(), work);
        assert_eq!(flow.work(work).unwrap().request, Some(submitted.request()));
    }

    #[test]
    fn conflicting_external_request_never_becomes_a_second_bridge_receipt() {
        let (input, mut flow, mut adapter, resource) = input(
            FidelityMode::Micro,
            TransitIntent::Zero,
            SeedPurpose::Service,
            false,
        );
        flow.register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        let created = must_create(prepared.create(&mut flow));
        let bound = must_bind(created.bind(&flow));
        let work = bound.work();
        let draw_position = bound.draw_position();
        let owner = flow.work(work).unwrap().owner;
        let external = flow
            .acquire(resource)
            .owner(owner)
            .timed_work(work)
            .submit()
            .unwrap();
        let before_scheduler = flow.budget_snapshot().scheduler;
        let failure = match bound.submit(&mut flow) {
            Ok(_) => panic!("a pre-existing request must block a bridge receipt"),
            Err(failure) => failure,
        };
        assert_eq!(failure.error, BridgeError::ConflictingSubmission);
        assert_eq!(failure.bound.work(), work);
        assert_eq!(failure.bound.draw_position(), draw_position);
        assert_eq!(flow.work(work).unwrap().request, Some(external));
        assert_eq!(flow.budget_snapshot().scheduler, before_scheduler);
        assert_eq!(flow.request(external).unwrap().work, Some(work));
    }
}
