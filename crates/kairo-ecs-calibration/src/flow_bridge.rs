//! Private, experimental calibration-to-Flow admission bridge.
//!
//! Intrinsic service sampling owns one Service stream. Nonzero Micro transit is
//! scheduled through the ABM TransitContext; Macro and zero routes submit work
//! without creating a transit carrier or event.

use crate::seed_map::{CalibrationStream, CalibrationStreamKey, SeedPurpose};
use crate::work_duration::{IntrinsicWorkProvider, SampledWorkDuration, WorkDurationError};
use kairo_ecs_abm::spatial::{MovementProfile, NodeId, TransitError, TransitGraphV1};
use kairo_ecs_abm::{
    schedule_transit_control, schedule_transit_start, TransitContext, TransitPhase,
};
use kairo_ecs_des::fidelity::{
    FidelityAdapter, FidelityAdmissionPermit, FidelityDecision, FidelityError, FidelityMode,
};
use kairo_ecs_des::{
    FlowAcquireCommand, FlowBatchReceipt, FlowDispatch, FlowDomainControl, FlowError, FlowRuntime,
    FlowRuntimeIdentity, PreemptionStrategy, RequestId, ResourceId, WorkId, WorkState,
};
use kairo_ecs_types::{EntityId, EventId, EventKind, SimDuration, SimTime};
use std::marker::PhantomData;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PreparationIdentity {
    pub(crate) owner: EntityId,
    pub(crate) subsystem: String,
    pub(crate) registration: String,
    pub(crate) stratum: String,
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

#[derive(Clone)]
pub(crate) enum TransitRequest {
    Zero,
    Route {
        graph: Arc<TransitGraphV1>,
        origin: NodeId,
        destination: NodeId,
        profile: MovementProfile,
        ticks_per_second: u64,
        carrier_actor: EntityId,
        carrier_registration: String,
        kind: EventKind,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TransitObservation {
    Progress,
    Paused,
    Resumed,
    IgnoredStale,
    Rejected,
    Arrived,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum BridgeError {
    Fidelity(FidelityError),
    Duration(WorkDurationError),
    Transit(TransitError),
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
    pub(crate) transit: TransitRequest,
}

impl<T: Clone, C: 'static> WorkPreparationInput<T, C> {
    pub(crate) fn new(
        identity: PreparationIdentity,
        service_stream: CalibrationStream,
        expected_service_key: CalibrationStreamKey,
        template: T,
        make_context: fn(&T) -> C,
        acquire: AcquireIntent,
        transit: TransitRequest,
    ) -> Self {
        Self {
            owner: identity.owner,
            subsystem: identity.subsystem,
            stratum: identity.stratum,
            expected_service_key,
            service_stream,
            template,
            registration: identity.registration,
            make_context,
            acquire,
            transit,
        }
    }
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
    transit: TransitRequest,
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
    transit: TransitRequest,
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
    transit: TransitRequest,
    runtime: FlowRuntimeIdentity,
    work: WorkId,
    carrier: Option<WorkId>,
    carrier_actor: Option<EntityId>,
    kind: Option<EventKind>,
    pending_event: Option<EventId>,
    pending_priority: Option<i32>,
    owned_events: Vec<EventId>,
    stale_events: Vec<EventId>,
    consumed_events: Vec<EventId>,
    controls: Vec<(EventId, FlowDomainControl)>,
    retryable: Option<FlowDispatch>,
    arrival_request: Option<RequestId>,
    arrival_at: Option<SimTime>,
    _restart_types: PhantomData<fn() -> (T, C)>,
}

/// One-shot, same-process continuation for one already-bound bridge item.
///
/// Owning the runtime and adapter prevents their engine APIs from advancing
/// while detached. The private fields and consuming resume make the handoff
/// opaque and non-forkable.
pub(crate) struct BoundWorkContinuation<T: Clone, C: 'static> {
    flow: FlowRuntime,
    adapter: FidelityAdapter,
    bound: BoundIntrinsicWork<T, C>,
}

impl<T: Clone + 'static, C: 'static> BoundWorkContinuation<T, C> {
    #[allow(clippy::result_large_err)]
    pub(crate) fn capture(
        flow: FlowRuntime,
        adapter: FidelityAdapter,
        bound: BoundIntrinsicWork<T, C>,
    ) -> Result<
        Self,
        (
            FlowRuntime,
            FidelityAdapter,
            BoundIntrinsicWork<T, C>,
            BridgeError,
        ),
    > {
        let valid_runtime = bound.runtime == flow.identity();
        let valid_decision = adapter.decision(bound.work) == Some(&bound.decision);
        if !valid_runtime || !valid_decision {
            return Err((
                flow,
                adapter,
                bound,
                BridgeError::Fidelity(FidelityError::InvalidWork),
            ));
        }
        Ok(Self {
            flow,
            adapter,
            bound,
        })
    }

    pub(crate) fn resume(self) -> (FlowRuntime, FidelityAdapter, BoundIntrinsicWork<T, C>) {
        (self.flow, self.adapter, self.bound)
    }
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

        if decision.mode == FidelityMode::Micro {
            let zero_route = match &self.transit {
                TransitRequest::Zero => false,
                TransitRequest::Route {
                    graph,
                    origin,
                    destination,
                    profile,
                    ticks_per_second,
                    ..
                } => match graph.route(*origin, *destination, profile, *ticks_per_second) {
                    Ok(route) => route.duration() == SimDuration::ZERO,
                    Err(error) => {
                        drop(permit);
                        return Err(PrepareFailure {
                            input: self,
                            error: BridgeError::Transit(error),
                        });
                    }
                },
            };
            if zero_route {
                self.transit = TransitRequest::Zero;
            }
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
            transit: self.transit,
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

    pub(crate) fn service_draw_position(&self) -> u64 {
        self.service_stream.draw_position()
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
            transit,
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
                transit,
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
                    transit,
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
                transit: self.transit,
                runtime: flow.identity(),
                work: self.work,
                carrier: None,
                carrier_actor: None,
                kind: None,
                pending_event: None,
                pending_priority: None,
                owned_events: Vec::new(),
                stale_events: Vec::new(),
                consumed_events: Vec::new(),
                controls: Vec::new(),
                retryable: None,
                arrival_request: None,
                arrival_at: None,
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

    pub(crate) fn service_draw_position(&self) -> u64 {
        self.draw_position()
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

    pub(crate) fn start_transit(&mut self, flow: &mut FlowRuntime) -> Result<EventId, BridgeError> {
        if self.runtime != flow.identity() {
            return Err(BridgeError::Fidelity(FidelityError::InvalidWork));
        }
        if self.decision.mode != FidelityMode::Micro {
            return Err(BridgeError::InvalidDispatch);
        }
        let (graph, origin, destination, profile, tps, actor, registration, kind) =
            match &self.transit {
                TransitRequest::Route {
                    graph,
                    origin,
                    destination,
                    profile,
                    ticks_per_second,
                    carrier_actor,
                    carrier_registration,
                    kind,
                } => (
                    graph,
                    *origin,
                    *destination,
                    profile,
                    *ticks_per_second,
                    *carrier_actor,
                    carrier_registration.as_str(),
                    *kind,
                ),
                TransitRequest::Zero => return Err(BridgeError::InvalidDispatch),
            };
        if self.pending_event.is_some() {
            return Err(BridgeError::InvalidDispatch);
        }
        let carrier = if let Some(carrier) = self.carrier {
            if self.carrier_actor != Some(actor) || self.kind != Some(kind) {
                return Err(BridgeError::InvalidDispatch);
            }
            carrier
        } else {
            let route = graph
                .route(origin, destination, profile, tps)
                .map_err(BridgeError::Transit)?;
            if route.duration() == SimDuration::ZERO {
                return Err(BridgeError::InvalidDispatch);
            }
            let acquire = FlowAcquireCommand {
                resource: self.acquire.resource,
                owner: self.acquire.owner,
                work: Some(self.work),
                at: self.acquire.at,
                priority_level: self.acquire.priority_level,
                deadline: self.acquire.deadline,
                scheduler_priority: self.acquire.scheduler_priority,
                timed: true,
                can_preempt: self.acquire.can_preempt,
                preemptible: self.acquire.preemptible,
            };
            let context = TransitContext::new(flow, route, acquire, self.acquire.at)
                .map_err(BridgeError::Transit)?;
            let carrier = flow
                .create_actor_domain_context(actor, registration, kind, context)
                .map_err(BridgeError::Flow)?;
            self.carrier = Some(carrier);
            self.carrier_actor = Some(actor);
            self.kind = Some(kind);
            self.pending_priority = Some(self.acquire.scheduler_priority);
            carrier
        };
        let event = schedule_transit_start(
            flow,
            carrier,
            kind,
            self.acquire.at,
            self.acquire.scheduler_priority,
        )
        .map_err(BridgeError::Flow)?;
        self.pending_event = Some(event);
        self.owned_events.push(event);
        Ok(event)
    }

    pub(crate) fn schedule_transit_control(
        &mut self,
        flow: &mut FlowRuntime,
        action: FlowDomainControl,
        at: SimTime,
        priority: i32,
    ) -> Result<EventId, BridgeError> {
        if self.runtime != flow.identity() {
            return Err(BridgeError::Fidelity(FidelityError::InvalidWork));
        }
        let (Some(carrier), Some(kind)) = (self.carrier, self.kind) else {
            return Err(BridgeError::InvalidDispatch);
        };
        let event = schedule_transit_control(flow, carrier, kind, action, at, priority)
            .map_err(BridgeError::Flow)?;
        self.controls.push((event, action));
        self.owned_events.push(event);
        Ok(event)
    }

    pub(crate) fn observe_transit_dispatch(
        &mut self,
        flow: &FlowRuntime,
        dispatch: &FlowDispatch,
    ) -> Result<TransitObservation, BridgeError> {
        if self.runtime != flow.identity() {
            return Err(BridgeError::Fidelity(FidelityError::InvalidWork));
        }
        let (Some(carrier), Some(_kind), Some(pending)) =
            (self.carrier, self.kind, self.pending_event)
        else {
            return Err(BridgeError::InvalidDispatch);
        };
        if dispatch.at != flow.now()
            || !self.owned_events.contains(&dispatch.event)
            || self.consumed_events.contains(&dispatch.event)
        {
            return Err(BridgeError::InvalidDispatch);
        }
        if let Some(index) = self
            .controls
            .iter()
            .position(|(event, _)| *event == dispatch.event)
        {
            let action = self.controls[index].1;
            let [FlowBatchReceipt::Accepted(admissions)] = dispatch.callback_batches.as_slice()
            else {
                return match dispatch.callback_batches.as_slice() {
                    [FlowBatchReceipt::Rejected(rejection)] => {
                        Err(BridgeError::Flow(rejection.error))
                    }
                    _ => Err(BridgeError::InvalidDispatch),
                };
            };
            let context = flow
                .work_context::<TransitContext>(carrier)
                .map_err(BridgeError::Flow)?;
            if context.service_work() != self.work {
                return Err(BridgeError::InvalidDispatch);
            }
            let scheduled = match action {
                FlowDomainControl::Pause => {
                    if context.phase() != TransitPhase::Paused || !admissions.is_empty() {
                        return Err(BridgeError::InvalidDispatch);
                    }
                    None
                }
                FlowDomainControl::Resume => {
                    if !matches!(context.phase(), TransitPhase::Ready | TransitPhase::Moving) {
                        return Err(BridgeError::InvalidDispatch);
                    }
                    match admissions.as_slice() {
                        [] => None,
                        [admission]
                            if admission.request.is_none() && admission.event != dispatch.event =>
                        {
                            Some(admission.event)
                        }
                        _ => return Err(BridgeError::InvalidDispatch),
                    }
                }
            };
            self.controls.remove(index);
            self.consumed_events.push(dispatch.event);
            if let Some(event) = scheduled {
                self.pending_event = Some(event);
                self.owned_events.push(event);
            }
            return Ok(match action {
                FlowDomainControl::Pause => TransitObservation::Paused,
                FlowDomainControl::Resume => TransitObservation::Resumed,
            });
        }
        if dispatch.event != pending {
            if self.stale_events.contains(&dispatch.event) {
                self.consumed_events.push(dispatch.event);
                return Ok(TransitObservation::IgnoredStale);
            }
            return Err(BridgeError::InvalidDispatch);
        }
        if matches!(
            dispatch.callback_batches.as_slice(),
            [FlowBatchReceipt::Rejected(_)]
        ) {
            if self.carrier_actor.is_none()
                || self.pending_priority.is_none()
                || flow
                    .work_context::<TransitContext>(carrier)
                    .map_err(BridgeError::Flow)?
                    .service_work()
                    != self.work
            {
                return Err(BridgeError::InvalidDispatch);
            }
            TransitContext::validate_retry_dispatch(flow, carrier, dispatch)
                .map_err(|_| BridgeError::InvalidDispatch)?;
            self.retryable = Some(dispatch.clone());
            return Ok(TransitObservation::Rejected);
        }
        let [FlowBatchReceipt::Accepted(admissions)] = dispatch.callback_batches.as_slice() else {
            return Err(BridgeError::InvalidDispatch);
        };
        let context = flow
            .work_context::<TransitContext>(carrier)
            .map_err(BridgeError::Flow)?;
        if context.service_work() != self.work {
            return Err(BridgeError::InvalidDispatch);
        }
        let phase = context.phase();
        let ticket = match phase {
            TransitPhase::Moving => context.next_progress_ticket(),
            TransitPhase::Arrived => context.arrival_ticket(),
            _ => None,
        };
        if let Some(ticket) = ticket {
            let [admission] = admissions.as_slice() else {
                return Err(BridgeError::InvalidDispatch);
            };
            if admission.ticket != ticket {
                return Err(BridgeError::InvalidDispatch);
            }
            if admission.event == dispatch.event {
                return Err(BridgeError::InvalidDispatch);
            }
            if phase == TransitPhase::Arrived {
                let request = admission.request.ok_or(BridgeError::InvalidDispatch)?;
                let saved = flow.request(request).map_err(BridgeError::Flow)?;
                if saved.work != Some(self.work)
                    || saved.owner != self.acquire.owner
                    || saved.resource != self.acquire.resource
                    || !saved.timed
                    || saved.submitted_at != dispatch.at
                    || saved.priority_level != self.acquire.priority_level
                    || saved.deadline != self.acquire.deadline
                    || saved.can_preempt != self.acquire.can_preempt
                    || saved.preemptible != self.acquire.preemptible
                    || flow.work(self.work).map_err(BridgeError::Flow)?.request != Some(request)
                {
                    return Err(BridgeError::InvalidDispatch);
                }
                self.consumed_events.push(dispatch.event);
                self.pending_event = Some(admission.event);
                self.owned_events.push(admission.event);
                self.arrival_request = Some(request);
                self.arrival_at = Some(dispatch.at);
                return Ok(TransitObservation::Arrived);
            }
            if admission.request.is_some() {
                return Err(BridgeError::InvalidDispatch);
            }
            self.consumed_events.push(dispatch.event);
            self.pending_event = Some(admission.event);
            self.owned_events.push(admission.event);
            return Ok(TransitObservation::Progress);
        }
        if !admissions.is_empty()
            || context.phase() != TransitPhase::Paused
            || context.expects_event(dispatch.event, dispatch.at)
        {
            return Err(BridgeError::InvalidDispatch);
        }
        self.consumed_events.push(dispatch.event);
        self.stale_events.push(dispatch.event);
        Ok(TransitObservation::IgnoredStale)
    }

    pub(crate) fn retry_transit(
        &mut self,
        flow: &mut FlowRuntime,
        rejected: &FlowDispatch,
    ) -> Result<EventId, BridgeError> {
        if self.runtime != flow.identity() {
            return Err(BridgeError::Fidelity(FidelityError::InvalidWork));
        }
        let (Some(carrier), Some(kind), Some(pending), Some(priority), Some(saved)) = (
            self.carrier,
            self.kind,
            self.pending_event,
            self.pending_priority,
            self.retryable.as_ref(),
        ) else {
            return Err(BridgeError::InvalidDispatch);
        };
        let TransitRequest::Route {
            carrier_actor,
            kind: expected_kind,
            ..
        } = &self.transit
        else {
            return Err(BridgeError::InvalidDispatch);
        };
        if self.carrier_actor != Some(*carrier_actor)
            || kind != *expected_kind
            || priority != self.acquire.scheduler_priority
            || rejected != saved
            || rejected.event != pending
            || self.consumed_events.contains(&pending)
            || !matches!(
                rejected.callback_batches.as_slice(),
                [FlowBatchReceipt::Rejected(_)]
            )
        {
            return Err(BridgeError::InvalidDispatch);
        }
        let context = flow
            .work_context::<TransitContext>(carrier)
            .map_err(BridgeError::Flow)?;
        if context.service_work() != self.work {
            return Err(BridgeError::InvalidDispatch);
        }
        let event = TransitContext::schedule_transit_retry(flow, carrier, kind, rejected, priority)
            .map_err(BridgeError::Flow)?;
        self.consumed_events.push(pending);
        self.pending_event = Some(event);
        self.owned_events.push(event);
        self.retryable = None;
        Ok(event)
    }

    pub(crate) fn finish_transit(
        self,
        flow: &FlowRuntime,
    ) -> Result<SubmittedIntrinsicWork<T, C>, BridgeError> {
        if self.runtime != flow.identity() {
            return Err(BridgeError::Fidelity(FidelityError::InvalidWork));
        }
        let (Some(carrier), Some(request), Some(at)) =
            (self.carrier, self.arrival_request, self.arrival_at)
        else {
            return Err(BridgeError::InvalidDispatch);
        };
        let context = flow
            .work_context::<TransitContext>(carrier)
            .map_err(BridgeError::Flow)?;
        if context.service_work() != self.work || context.phase() != TransitPhase::Arrived {
            return Err(BridgeError::InvalidDispatch);
        }
        let saved = flow.request(request).map_err(BridgeError::Flow)?;
        if saved.resource != self.acquire.resource
            || saved.owner != self.acquire.owner
            || saved.work != Some(self.work)
            || !saved.timed
            || saved.submitted_at != at
            || saved.priority_level != self.acquire.priority_level
            || saved.deadline != self.acquire.deadline
            || saved.can_preempt != self.acquire.can_preempt
            || saved.preemptible != self.acquire.preemptible
            || flow.work(self.work).map_err(BridgeError::Flow)?.request != Some(request)
        {
            return Err(BridgeError::InvalidDispatch);
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
        if self.decision.mode == FidelityMode::Micro
            && matches!(&self.transit, TransitRequest::Route { .. })
        {
            return Err(SubmitFailure {
                bound: self,
                error: BridgeError::Flow(FlowError::InvalidState),
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

    pub(crate) fn service_draw_position(&self) -> u64 {
        self.draw_position()
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
    use kairo_ecs_abm::spatial::{EdgeId, MovementModeId, TransitEdge};
    use kairo_ecs_abm::{register_transit_context, register_transit_context_reject_first_for_test};
    use kairo_ecs_des::fidelity::FidelityPolicy;
    use kairo_ecs_des::{FlowBatchRejection, FlowDispatch, RequestState, WorkHandlers};

    #[derive(Clone, Copy)]
    enum TransitIntent {
        Zero,
        Route,
    }

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
        let carrier_actor = flow.spawn_actor().unwrap();
        let resource = flow.create_resource(1).unwrap();

        let mut seed_map = CalibrationSeedMap::new(1, "bridge-test", 19).unwrap();
        let key = seed_map
            .key_for("paired", 0, "case-a", "task-a", SeedPurpose::Service)
            .unwrap();
        let stream_task = if mismatch_key { "task-b" } else { "task-a" };
        let service_stream = seed_map
            .stream_for("paired", 0, "case-a", stream_task, purpose)
            .unwrap();
        let transit = match transit {
            TransitIntent::Zero => TransitRequest::Zero,
            TransitIntent::Route => {
                let node = |n| NodeId::new(n);
                let mode = MovementModeId::new("walk").unwrap();
                TransitRequest::Route {
                    graph: Arc::new(
                        TransitGraphV1::new(
                            1,
                            vec![node(1), node(2)],
                            vec![TransitEdge {
                                id: EdgeId::new(1),
                                from: node(1),
                                to: node(2),
                                length_mm: 1,
                                allowed_modes: vec![mode],
                            }],
                        )
                        .unwrap(),
                    ),
                    origin: node(1),
                    destination: node(2),
                    profile: MovementProfile::new("walk", 1).unwrap(),
                    ticks_per_second: 1,
                    carrier_actor,
                    carrier_registration: "bridge.transit".to_owned(),
                    kind: EventKind::custom(0xC20),
                }
            }
        };
        let input = WorkPreparationInput::new(
            PreparationIdentity {
                owner,
                subsystem: "assessment".to_owned(),
                registration: "bridge.context".to_owned(),
                stratum: "triage".to_owned(),
            },
            service_stream,
            key,
            42,
            make_context,
            AcquireIntent {
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
        );
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

    fn bound_route_with_adapter() -> (FlowRuntime, FidelityAdapter, BoundIntrinsicWork<u32, u32>) {
        let (input, mut flow, mut adapter, _) = input(
            FidelityMode::Micro,
            TransitIntent::Route,
            SeedPurpose::Service,
            false,
        );
        register_transit_context(&mut flow, "bridge.transit", EventKind::custom(0xC20)).unwrap();
        flow.register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        let created = must_create(prepared.create(&mut flow));
        let bound = must_bind(created.bind(&flow));
        (flow, adapter, bound)
    }

    fn bound_simple_with_adapter(
        mode: FidelityMode,
        transit: TransitIntent,
    ) -> (FlowRuntime, FidelityAdapter, BoundIntrinsicWork<u32, u32>) {
        let (input, mut flow, mut adapter, _) = input(mode, transit, SeedPurpose::Service, false);
        flow.register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        let created = must_create(prepared.create(&mut flow));
        let bound = must_bind(created.bind(&flow));
        (flow, adapter, bound)
    }

    fn bound_route_at(
        start: SimTime,
        edge_count: u32,
    ) -> (BoundIntrinsicWork<u32, u32>, FlowRuntime) {
        bound_route_at_with_registration(start, edge_count, register_transit_context)
    }

    fn bound_route_at_with_registration(
        start: SimTime,
        edge_count: u32,
        register: fn(&mut FlowRuntime, &str, EventKind) -> Result<(), FlowError>,
    ) -> (BoundIntrinsicWork<u32, u32>, FlowRuntime) {
        let (mut input, mut flow, mut adapter, _) = input(
            FidelityMode::Micro,
            TransitIntent::Route,
            SeedPurpose::Service,
            false,
        );
        input.acquire.at = start;
        let carrier_actor = match &input.transit {
            TransitRequest::Route { carrier_actor, .. } => *carrier_actor,
            TransitRequest::Zero => unreachable!(),
        };
        let nodes = (1..=u64::from(edge_count) + 1)
            .map(NodeId::new)
            .collect::<Vec<_>>();
        let mode = MovementModeId::new("walk").unwrap();
        let edges = (0..edge_count)
            .map(|index| TransitEdge {
                id: EdgeId::new(index as u64 + 1),
                from: nodes[index as usize],
                to: nodes[index as usize + 1],
                length_mm: 1,
                allowed_modes: vec![mode.clone()],
            })
            .collect();
        input.transit = TransitRequest::Route {
            graph: Arc::new(TransitGraphV1::new(1, nodes.clone(), edges).unwrap()),
            origin: nodes[0],
            destination: *nodes.last().unwrap(),
            profile: MovementProfile::new("walk", 1).unwrap(),
            ticks_per_second: 1,
            carrier_actor,
            carrier_registration: "bridge.transit".to_owned(),
            kind: EventKind::custom(0xC20),
        };
        register(&mut flow, "bridge.transit", EventKind::custom(0xC20)).unwrap();
        flow.register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        let created = must_create(prepared.create(&mut flow));
        (must_bind(created.bind(&flow)), flow)
    }

    fn assert_overflow_rejection(dispatch: &FlowDispatch) {
        assert_eq!(dispatch.error, Some(FlowError::CounterOverflow));
        assert!(matches!(
            dispatch.callback_batches.as_slice(),
            [FlowBatchReceipt::Rejected(rejection)]
                if rejection.error == FlowError::CounterOverflow
                    && rejection.failed_ticket.is_none()
        ));
    }

    #[test]
    fn owning_continuation_preserves_macro_and_zero_micro_values_and_service_draws() {
        for (mode, transit) in [
            (FidelityMode::Macro, TransitIntent::Route),
            (FidelityMode::Micro, TransitIntent::Zero),
        ] {
            let (flow, mut adapter, bound) = bound_simple_with_adapter(mode, transit);
            let (control_flow, mut control_adapter, mut control_bound) =
                bound_simple_with_adapter(mode, transit);
            let identity = flow.identity();
            let work = bound.work();
            let progress = flow.work_progress(work).unwrap();
            let stream_position = bound.service_draw_position();
            adapter.stage_policy(FidelityPolicy::new(2, Some(FidelityMode::Micro)).unwrap());
            control_adapter
                .stage_policy(FidelityPolicy::new(2, Some(FidelityMode::Micro)).unwrap());

            let continuation = BoundWorkContinuation::capture(flow, adapter, bound)
                .ok()
                .expect("valid bound bridge values capture");
            let (flow, mut adapter, mut bound) = continuation.resume();
            assert_eq!(flow.identity(), identity);
            assert_eq!(flow.work_progress(work).unwrap(), progress);
            assert_eq!(adapter.decision(work), Some(&bound.decision));
            assert_eq!(bound.service_draw_position(), stream_position);
            assert_eq!(flow.budget_snapshot(), control_flow.budget_snapshot());
            assert_eq!(
                bound.service_stream.next_u64().unwrap(),
                control_bound.service_stream.next_u64().unwrap()
            );
            assert_eq!(
                bound.service_draw_position(),
                control_bound.service_draw_position()
            );
            assert_eq!(
                adapter.apply_at_boundary(&flow),
                Err(FidelityError::BusyBoundary)
            );
            assert_eq!(
                control_adapter.apply_at_boundary(&control_flow),
                Err(FidelityError::BusyBoundary)
            );
            assert_eq!(adapter, control_adapter);
        }
    }

    #[test]
    fn owning_continuation_preserves_started_and_observed_paused_transit() {
        let (mut flow, adapter, mut bound) = bound_route_with_adapter();
        let (mut control_flow, control_adapter, mut control_bound) = bound_route_with_adapter();
        let identity = flow.identity();
        let work = bound.work();
        let start = bound.start_transit(&mut flow).unwrap();
        let control_start = control_bound.start_transit(&mut control_flow).unwrap();
        assert_eq!(start, control_start);

        let continuation = BoundWorkContinuation::capture(flow, adapter, bound)
            .ok()
            .expect("same-runtime bridge values capture after transit start");
        let (mut flow, adapter, mut bound) = continuation.resume();
        assert_eq!(flow.identity(), identity);
        assert_eq!(adapter.decision(work), Some(&bound.decision));
        assert_eq!(bound.pending_event, Some(start));
        assert_eq!(flow.budget_snapshot(), control_flow.budget_snapshot());

        let start_dispatch = flow.step().unwrap().unwrap();
        let control_start_dispatch = control_flow.step().unwrap().unwrap();
        assert_eq!(start_dispatch, control_start_dispatch);
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &start_dispatch),
            Ok(TransitObservation::Progress)
        );
        assert_eq!(
            control_bound.observe_transit_dispatch(&control_flow, &control_start_dispatch),
            Ok(TransitObservation::Progress)
        );
        let now = flow.now();
        let pause = bound
            .schedule_transit_control(&mut flow, FlowDomainControl::Pause, now, 7)
            .unwrap();
        let control_now = control_flow.now();
        let control_pause = control_bound
            .schedule_transit_control(&mut control_flow, FlowDomainControl::Pause, control_now, 7)
            .unwrap();
        assert_eq!(pause, control_pause);
        let pause_dispatch = flow.step().unwrap().unwrap();
        let control_pause_dispatch = control_flow.step().unwrap().unwrap();
        assert_eq!(pause_dispatch, control_pause_dispatch);
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &pause_dispatch),
            Ok(TransitObservation::Paused)
        );
        assert_eq!(
            control_bound.observe_transit_dispatch(&control_flow, &control_pause_dispatch),
            Ok(TransitObservation::Paused)
        );
        let carrier = bound.carrier.unwrap();
        let progress = flow.work_progress(work).unwrap();
        let context = flow.work_context::<TransitContext>(carrier).unwrap();
        assert_eq!(context.phase(), TransitPhase::Paused);
        let control_context = control_flow
            .work_context::<TransitContext>(control_bound.carrier.unwrap())
            .unwrap();
        assert_eq!(context.phase(), control_context.phase());
        assert_eq!(
            context.progress_at(flow.now()),
            control_context.progress_at(flow.now())
        );

        let continuation = BoundWorkContinuation::capture(flow, adapter, bound)
            .ok()
            .expect("same-runtime bound bridge values capture after observed pause");
        let (mut flow, adapter, mut bound) = continuation.resume();
        assert_eq!(flow.identity(), identity);
        assert_eq!(flow.work_progress(work).unwrap(), progress);
        assert_eq!(adapter.decision(work), Some(&bound.decision));
        assert_eq!(bound.owned_events, control_bound.owned_events);
        assert_eq!(bound.consumed_events, control_bound.consumed_events);
        assert_eq!(flow.budget_snapshot(), control_flow.budget_snapshot());
        assert_eq!(
            bound.service_stream.next_u64().unwrap(),
            control_bound.service_stream.next_u64().unwrap()
        );

        // Both paused continuations resume through the same accepted arrival
        // callback exactly once, with matching bridge receipts and ownership.
        let resume_at = flow.now();
        let resume = bound
            .schedule_transit_control(&mut flow, FlowDomainControl::Resume, resume_at, 7)
            .unwrap();
        let control_resume_at = control_flow.now();
        let control_resume = control_bound
            .schedule_transit_control(
                &mut control_flow,
                FlowDomainControl::Resume,
                control_resume_at,
                7,
            )
            .unwrap();
        assert_eq!(resume, control_resume);
        let dispatch = flow.step().unwrap().unwrap();
        let control_dispatch = control_flow.step().unwrap().unwrap();
        assert_eq!(dispatch, control_dispatch);
        assert_eq!(dispatch.event, resume);
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &dispatch),
            Ok(TransitObservation::Resumed)
        );
        assert_eq!(
            control_bound.observe_transit_dispatch(&control_flow, &control_dispatch),
            Ok(TransitObservation::Resumed)
        );
        let mut arrived = false;
        while !arrived {
            let dispatch = flow.step().unwrap().unwrap();
            let control_dispatch = control_flow.step().unwrap().unwrap();
            assert_eq!(dispatch, control_dispatch);
            let observation = bound.observe_transit_dispatch(&flow, &dispatch).unwrap();
            let control_observation = control_bound
                .observe_transit_dispatch(&control_flow, &control_dispatch)
                .unwrap();
            assert_eq!(observation, control_observation);
            arrived = observation == TransitObservation::Arrived;
        }
        assert_eq!(bound.owned_events.len(), 5);
        assert_eq!(bound.consumed_events.len(), 4);
        assert!(bound.arrival_request.is_some());
        assert_eq!(bound.owned_events.len(), control_bound.owned_events.len());
        assert_eq!(
            bound.consumed_events.len(),
            control_bound.consumed_events.len()
        );
        assert_eq!(bound.owned_events, control_bound.owned_events);
        assert_eq!(bound.consumed_events, control_bound.consumed_events);
        assert_eq!(adapter.decision(work), control_adapter.decision(work));
    }

    #[test]
    fn continuation_capture_mismatch_returns_the_original_values() {
        let (flow, adapter, bound) =
            bound_simple_with_adapter(FidelityMode::Macro, TransitIntent::Route);
        let original_identity = flow.identity();
        let original_work = bound.work();
        let foreign_flow = FlowRuntime::new();
        let foreign_identity = foreign_flow.identity();
        let result = BoundWorkContinuation::capture(foreign_flow, adapter, bound);
        let (returned_flow, returned_adapter, returned_bound, error) = match result {
            Err(values) => values,
            Ok(_) => panic!("foreign runtime must fail capture validation"),
        };
        assert_eq!(error, BridgeError::Fidelity(FidelityError::InvalidWork));
        assert_eq!(returned_flow.identity(), foreign_identity);
        assert_eq!(
            returned_adapter.decision(original_work),
            Some(&returned_bound.decision)
        );
        assert_eq!(returned_bound.runtime, original_identity);
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
        assert_eq!(
            macro_submitted.expected_service_key,
            micro_submitted.expected_service_key
        );
        assert_eq!(
            macro_submitted.service_stream.derived_seed(),
            micro_submitted.service_stream.derived_seed()
        );
        assert_eq!(
            macro_submitted.expected_service_key,
            macro_submitted.service_stream.key()
        );
        assert_eq!(
            micro_submitted.expected_service_key,
            micro_submitted.service_stream.key()
        );
        assert!(macro_submitted.service_identity_matches());
        assert!(micro_submitted.service_identity_matches());
        assert_eq!(macro_submitted.decision().mode, FidelityMode::Macro);
        assert_eq!(micro_submitted.decision().mode, FidelityMode::Micro);
        assert_eq!(macro_submitted.acquire_intent().scheduler_priority, 7);
        assert_eq!(micro_submitted.acquire_intent().scheduler_priority, 7);
        assert_eq!(macro_flow.budget_snapshot().scheduler.scheduled_events, 1);
        assert_eq!(micro_flow.budget_snapshot().scheduler.scheduled_events, 1);

        let macro_request = {
            let flow = &mut macro_flow;
            let spec = flow.work(macro_submitted.work()).unwrap();
            assert_eq!(spec.context_type_key, "bridge.context");
            assert_eq!(spec.original_duration, macro_submitted.sampled_duration());
            assert_eq!(spec.request, Some(macro_submitted.request()));
            let request = flow.request(macro_submitted.request()).unwrap();
            assert_eq!(request.resource, macro_resource);
            assert_eq!(request.owner, spec.owner);
            assert_eq!(request.work, Some(macro_submitted.work()));
            assert!(request.timed);
            assert_eq!(request.priority_level, 3);
            assert_eq!(request.deadline, None);
            assert!(!request.can_preempt);
            assert_eq!(request.state, RequestState::Pending);
            assert!(flow.step().unwrap().is_some());
            assert_eq!(
                flow.request(macro_submitted.request()).unwrap().state,
                RequestState::Active
            );
            request
        };
        let micro_request = {
            let flow = &mut micro_flow;
            let spec = flow.work(micro_submitted.work()).unwrap();
            assert_eq!(spec.context_type_key, "bridge.context");
            assert_eq!(spec.original_duration, micro_submitted.sampled_duration());
            assert_eq!(spec.request, Some(micro_submitted.request()));
            let request = flow.request(micro_submitted.request()).unwrap();
            assert_eq!(request.resource, micro_resource);
            assert_eq!(request.owner, spec.owner);
            assert_eq!(request.work, Some(micro_submitted.work()));
            assert!(request.timed);
            assert_eq!(request.priority_level, 3);
            assert_eq!(request.deadline, None);
            assert!(!request.can_preempt);
            assert_eq!(request.state, RequestState::Pending);
            assert!(flow.step().unwrap().is_some());
            assert_eq!(
                flow.request(micro_submitted.request()).unwrap().state,
                RequestState::Active
            );
            request
        };
        assert_eq!(macro_request, micro_request);

        let macro_run = macro_flow.run_for(10).unwrap();
        let micro_run = micro_flow.run_for(10).unwrap();
        assert!(!macro_run.budget_exhausted);
        assert!(!micro_run.budget_exhausted);
        assert_eq!(macro_run, micro_run);
        assert_eq!(macro_flow.now(), micro_flow.now());
        assert_eq!(
            macro_flow.now(),
            SimTime::from_ticks(0)
                .checked_add(macro_submitted.sampled_duration())
                .unwrap()
        );
        assert_eq!(
            macro_flow.work_progress(macro_submitted.work()).unwrap(),
            micro_flow.work_progress(micro_submitted.work()).unwrap()
        );
        assert_eq!(
            macro_flow
                .work_progress(macro_submitted.work())
                .unwrap()
                .state,
            WorkState::Completed
        );
        assert_eq!(
            macro_flow.request(macro_submitted.request()).unwrap().state,
            RequestState::Completed
        );
        assert_eq!(
            macro_flow.request(macro_submitted.request()).unwrap(),
            micro_flow.request(micro_submitted.request()).unwrap()
        );
        assert_eq!(
            macro_flow.budget_snapshot().scheduler.scheduled_events,
            micro_flow.budget_snapshot().scheduler.scheduled_events
        );
        assert_eq!(macro_flow.budget_snapshot().scheduler.scheduled_events, 2);
        // TransitContext contains deterministic route progress and command tickets,
        // but no CalibrationStream or other RNG state. This path submits only the
        // Service-purpose stream and schedules exactly the two Flow events above.
    }

    #[test]
    fn macro_and_micro_reconcile_queue_transit_and_service_elapsed_time() {
        let t = SimTime::from_ticks;
        let d = SimDuration::from_ticks;

        let (macro_input, mut macro_flow, mut macro_adapter, macro_resource) = input(
            FidelityMode::Macro,
            TransitIntent::Route,
            SeedPurpose::Service,
            false,
        );
        let macro_blocker = macro_flow.spawn_actor().unwrap();
        let macro_blocker_request = macro_flow
            .acquire(macro_resource)
            .owner(macro_blocker)
            .at(t(0))
            .submit()
            .unwrap();
        macro_flow.step().unwrap().unwrap();
        let macro_blocker_lease = macro_flow.resource(macro_resource).unwrap().allocations[0].lease;
        assert_eq!(
            macro_flow.request(macro_blocker_request).unwrap().state,
            RequestState::Active
        );

        let macro_prepared =
            must_prepare(macro_input.prepare(&macro_flow, &mut macro_adapter, &provider()));
        macro_flow
            .register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let macro_work = must_create(macro_prepared.create(&mut macro_flow));
        let macro_bound = must_bind(macro_work.bind(&macro_flow));
        let macro_submitted = must_submit(macro_bound.submit(&mut macro_flow));
        let service_duration = macro_submitted.sampled_duration();
        assert_eq!(
            macro_submitted.service_stream.derived_seed(),
            0x4e65_a0fe_ae85_3a9b
        );
        assert_eq!(service_duration, d(20));
        macro_flow.release(macro_blocker_lease, t(5)).unwrap();
        macro_flow.step().unwrap().unwrap();
        let macro_request = macro_flow.request(macro_submitted.request()).unwrap();
        let macro_submitted_at = macro_request.submitted_at;
        let macro_request_state = macro_request.state;
        assert_eq!(macro_submitted_at, t(0));
        assert_eq!(macro_request_state, RequestState::Queued);
        macro_flow.step().unwrap().unwrap();
        let macro_allocation = macro_flow
            .resource(macro_resource)
            .unwrap()
            .allocations
            .into_iter()
            .find(|allocation| allocation.request == macro_submitted.request())
            .unwrap();
        assert_eq!(macro_allocation.granted_at, t(5));
        let macro_completion = macro_allocation.completion_at.unwrap();
        assert_eq!(macro_flow.step().unwrap().unwrap().at, macro_completion);
        assert_eq!(
            macro_flow
                .work_progress(macro_submitted.work())
                .unwrap()
                .state,
            WorkState::Completed
        );

        let (mut micro_input, mut micro_flow, mut micro_adapter, micro_resource) = input(
            FidelityMode::Micro,
            TransitIntent::Route,
            SeedPurpose::Service,
            false,
        );
        let carrier_actor = match &micro_input.transit {
            TransitRequest::Route { carrier_actor, .. } => *carrier_actor,
            TransitRequest::Zero => unreachable!(),
        };
        let nodes = (1..=3).map(NodeId::new).collect::<Vec<_>>();
        let walk = MovementModeId::new("walk").unwrap();
        let edges = (0..2)
            .map(|index| TransitEdge {
                id: EdgeId::new(index + 1),
                from: nodes[index as usize],
                to: nodes[index as usize + 1],
                length_mm: 1,
                allowed_modes: vec![walk.clone()],
            })
            .collect();
        micro_input.transit = TransitRequest::Route {
            graph: Arc::new(TransitGraphV1::new(1, nodes.clone(), edges).unwrap()),
            origin: nodes[0],
            destination: *nodes.last().unwrap(),
            profile: MovementProfile::new("walk", 1).unwrap(),
            ticks_per_second: 1,
            carrier_actor,
            carrier_registration: "bridge.transit".to_owned(),
            kind: EventKind::custom(0xC20),
        };
        let micro_blocker = micro_flow.spawn_actor().unwrap();
        let micro_blocker_request = micro_flow
            .acquire(micro_resource)
            .owner(micro_blocker)
            .at(t(0))
            .submit()
            .unwrap();
        micro_flow.step().unwrap().unwrap();
        let micro_blocker_lease = micro_flow.resource(micro_resource).unwrap().allocations[0].lease;
        assert_eq!(
            micro_flow.request(micro_blocker_request).unwrap().state,
            RequestState::Active
        );

        let micro_prepared =
            must_prepare(micro_input.prepare(&micro_flow, &mut micro_adapter, &provider()));
        micro_flow
            .register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        register_transit_context(&mut micro_flow, "bridge.transit", EventKind::custom(0xC20))
            .unwrap();
        let micro_work = must_create(micro_prepared.create(&mut micro_flow));
        let mut micro_bound = must_bind(micro_work.bind(&micro_flow));
        micro_bound.start_transit(&mut micro_flow).unwrap();
        let carrier = micro_bound.carrier.unwrap();
        let start = micro_flow.step().unwrap().unwrap();
        assert_eq!(start.at, t(0));
        assert_eq!(
            micro_bound.observe_transit_dispatch(&micro_flow, &start),
            Ok(TransitObservation::Progress)
        );
        micro_flow.release(micro_blocker_lease, t(5)).unwrap();
        let progress = micro_flow.step().unwrap().unwrap();
        assert_eq!(progress.at, t(1));
        assert_eq!(
            micro_bound.observe_transit_dispatch(&micro_flow, &progress),
            Ok(TransitObservation::Progress)
        );
        let arrival = micro_flow.step().unwrap().unwrap();
        assert_eq!(arrival.at, t(2));
        assert_eq!(
            micro_bound.observe_transit_dispatch(&micro_flow, &arrival),
            Ok(TransitObservation::Arrived)
        );
        let transit = micro_flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .progress_at(t(2))
            .unwrap();
        assert_eq!(transit.useful_elapsed, d(2));
        let micro_submitted = micro_bound.finish_transit(&micro_flow).unwrap();
        assert_eq!(micro_submitted.sampled_duration(), service_duration);
        assert_eq!(
            micro_submitted.expected_service_key,
            macro_submitted.expected_service_key
        );
        assert_eq!(
            micro_submitted.service_stream.derived_seed(),
            macro_submitted.service_stream.derived_seed()
        );
        let submitted_dispatch = micro_flow.step().unwrap().unwrap();
        assert_eq!(submitted_dispatch.at, t(2));
        let micro_request = micro_flow.request(micro_submitted.request()).unwrap();
        let micro_submitted_at = micro_request.submitted_at;
        let micro_request_state = micro_request.state;
        assert_eq!(micro_submitted_at, t(2));
        assert_eq!(micro_request_state, RequestState::Queued);
        micro_flow.step().unwrap().unwrap();
        let micro_allocation = micro_flow
            .resource(micro_resource)
            .unwrap()
            .allocations
            .into_iter()
            .find(|allocation| allocation.request == micro_submitted.request())
            .unwrap();
        assert_eq!(micro_allocation.granted_at, t(5));
        let micro_completion = micro_allocation.completion_at.unwrap();
        assert_eq!(micro_flow.step().unwrap().unwrap().at, micro_completion);
        assert_eq!(
            micro_flow
                .work_progress(micro_submitted.work())
                .unwrap()
                .state,
            WorkState::Completed
        );

        let macro_queue = macro_allocation
            .granted_at
            .duration_since(macro_submitted_at)
            .unwrap();
        let micro_queue = micro_allocation
            .granted_at
            .duration_since(micro_submitted_at)
            .unwrap();
        let macro_service = macro_completion
            .duration_since(macro_allocation.granted_at)
            .unwrap();
        let micro_service = micro_completion
            .duration_since(micro_allocation.granted_at)
            .unwrap();
        assert_eq!(macro_queue, d(5));
        assert_eq!(micro_queue, d(3));
        assert_eq!(macro_service, service_duration);
        assert_eq!(micro_service, service_duration);
        assert_eq!(
            macro_completion,
            t(5).checked_add(service_duration).unwrap()
        );
        assert_eq!(micro_completion, macro_completion);
        assert_eq!(
            macro_completion.duration_since(t(0)).unwrap(),
            macro_queue.checked_add(macro_service).unwrap()
        );
        assert_eq!(
            micro_completion.duration_since(t(0)).unwrap(),
            transit
                .useful_elapsed
                .checked_add(micro_queue)
                .unwrap()
                .checked_add(micro_service)
                .unwrap()
        );
    }

    #[test]
    fn micro_route_is_prepared_and_samples_service_once() {
        let (input, flow, mut adapter, _) = input(
            FidelityMode::Micro,
            TransitIntent::Route,
            SeedPurpose::Service,
            false,
        );
        let prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        assert_eq!(prepared.draw_bounds(), (0, 1));
        assert_eq!(prepared.service_draw_position(), 1);
    }

    #[test]
    fn matching_rejected_transit_event_retries_once_and_replay_is_atomic() {
        let (input, mut flow, mut adapter, _) = input(
            FidelityMode::Micro,
            TransitIntent::Route,
            SeedPurpose::Service,
            false,
        );
        register_transit_context(&mut flow, "bridge.transit", EventKind::custom(0xC20)).unwrap();
        flow.register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        let created = must_create(prepared.create(&mut flow));
        let mut bound = must_bind(created.bind(&flow));
        let original = bound.start_transit(&mut flow).unwrap();
        let rejected = FlowDispatch {
            event: original,
            at: flow.now(),
            records: Vec::new(),
            error: None,
            callback_batches: vec![FlowBatchReceipt::Rejected(FlowBatchRejection {
                failed_ticket: None,
                error: FlowError::InvalidState,
            })],
        };
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &rejected),
            Ok(TransitObservation::Rejected)
        );
        let baseline = flow.budget_snapshot().scheduler;
        let mut wrong_event = rejected.clone();
        wrong_event.event = EventId::new(u64::MAX, u32::MAX);
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &wrong_event),
            Err(BridgeError::InvalidDispatch)
        );
        let mut forged_accepted = rejected.clone();
        forged_accepted.callback_batches = vec![FlowBatchReceipt::Accepted(Vec::new())];
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &forged_accepted),
            Err(BridgeError::InvalidDispatch)
        );
        let mut malformed = rejected.clone();
        malformed
            .callback_batches
            .push(FlowBatchReceipt::Accepted(Vec::new()));
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &malformed),
            Err(BridgeError::InvalidDispatch)
        );
        let mut foreign = FlowRuntime::new();
        assert_eq!(
            bound.retry_transit(&mut foreign, &rejected),
            Err(BridgeError::Fidelity(FidelityError::InvalidWork))
        );
        assert_eq!(flow.budget_snapshot().scheduler, baseline);
        let carrier = bound.carrier;
        let kind = bound.kind;
        let priority = bound.pending_priority;
        bound.carrier = Some(bound.work);
        assert!(bound.retry_transit(&mut flow, &rejected).is_err());
        bound.carrier = carrier;
        bound.kind = Some(EventKind::custom(0xC21));
        assert_eq!(
            bound.retry_transit(&mut flow, &rejected),
            Err(BridgeError::InvalidDispatch)
        );
        bound.kind = kind;
        bound.pending_priority = priority.map(|value| value + 1);
        assert_eq!(
            bound.retry_transit(&mut flow, &rejected),
            Err(BridgeError::InvalidDispatch)
        );
        bound.pending_priority = priority;
        assert_eq!(flow.budget_snapshot().scheduler, baseline);
        let carrier = bound.carrier.unwrap();
        let failed_schedule = TransitContext::schedule_transit_retry(
            &mut flow,
            carrier,
            EventKind::custom(0xC21),
            &rejected,
            7,
        );
        assert_eq!(failed_schedule, Err(FlowError::InvalidWork));
        assert_eq!(flow.budget_snapshot().scheduler, baseline);
        let before_retry = flow.budget_snapshot().scheduler;
        let replacement = bound.retry_transit(&mut flow, &rejected).unwrap();
        assert_ne!(replacement, original);
        let after_retry = flow.budget_snapshot().scheduler;
        assert_eq!(
            after_retry.scheduled_events,
            before_retry.scheduled_events + 1
        );
        assert_eq!(
            bound.retry_transit(&mut flow, &rejected),
            Err(BridgeError::InvalidDispatch)
        );
        assert_eq!(flow.budget_snapshot().scheduler, after_retry);
    }

    #[test]
    fn actual_transit_rejection_retries_to_one_accepted_arrival_and_request() {
        let (mut bound, mut flow) = bound_route_at_with_registration(
            SimTime::from_ticks(0),
            1,
            register_transit_context_reject_first_for_test,
        );
        let original = bound.start_transit(&mut flow).unwrap();
        let work_id = bound.work();
        let carrier = bound.carrier.unwrap();

        let rejected = flow
            .step()
            .unwrap()
            .expect("actual scheduled start callback");
        assert_eq!(rejected.event, original);
        assert_eq!(rejected.at, SimTime::from_ticks(0));
        assert_eq!(rejected.error, Some(FlowError::InvalidState));
        assert!(matches!(
            rejected.callback_batches.as_slice(),
            [FlowBatchReceipt::Rejected(rejection)]
                if rejection.error == FlowError::InvalidState
                    && rejection.failed_ticket.is_none()
        ));
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &rejected),
            Ok(TransitObservation::Rejected)
        );
        let ready = flow.work_context::<TransitContext>(carrier).unwrap();
        assert_eq!(ready.phase(), TransitPhase::Ready);
        assert!(ready.expects_event(original, SimTime::from_ticks(0)));
        assert_eq!(flow.work(bound.work()).unwrap().request, None);

        let replacement = bound.retry_transit(&mut flow, &rejected).unwrap();
        assert_ne!(replacement, original);
        assert!(flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .expects_event(replacement, SimTime::from_ticks(0)));
        let after_retry = flow.budget_snapshot().scheduler;
        assert_eq!(
            bound.retry_transit(&mut flow, &rejected),
            Err(BridgeError::InvalidDispatch)
        );
        assert_eq!(flow.budget_snapshot().scheduler, after_retry);
        assert_eq!(bound.owned_events, vec![original, replacement]);
        assert_eq!(bound.consumed_events, vec![original]);

        let retried_start = flow.step().unwrap().expect("actual retried start callback");
        assert_eq!(retried_start.event, replacement);
        assert!(matches!(
            retried_start.callback_batches.as_slice(),
            [FlowBatchReceipt::Accepted(admissions)] if admissions.len() == 1
        ));
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &retried_start),
            Ok(TransitObservation::Progress)
        );
        assert_eq!(bound.consumed_events, vec![original, replacement]);
        assert_eq!(flow.work(bound.work()).unwrap().request, None);
        assert_eq!(
            flow.work_context::<TransitContext>(carrier)
                .unwrap()
                .phase(),
            TransitPhase::Moving
        );

        let arrival = flow.step().unwrap().expect("actual route arrival callback");
        let arrival_event = bound.pending_event.unwrap();
        assert_ne!(arrival_event, original);
        assert_ne!(arrival_event, replacement);
        assert_eq!(arrival.event, arrival_event);
        assert!(matches!(
            arrival.callback_batches.as_slice(),
            [FlowBatchReceipt::Accepted(admissions)]
                if admissions.len() == 1 && admissions[0].request.is_some()
        ));
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &arrival),
            Ok(TransitObservation::Arrived)
        );
        let [FlowBatchReceipt::Accepted(admissions)] = arrival.callback_batches.as_slice() else {
            unreachable!("arrival was asserted accepted above")
        };
        let request_event = admissions[0].event;
        assert_eq!(bound.pending_event, Some(request_event));
        assert_ne!(request_event, arrival_event);
        assert_eq!(
            bound.owned_events,
            vec![original, replacement, arrival_event, request_event]
        );
        assert_eq!(
            bound.consumed_events,
            vec![original, replacement, arrival_event]
        );
        let request = admissions[0].request.unwrap();
        let saved_work = flow.work(work_id).unwrap();
        let saved_request = flow.request(request).unwrap();
        assert_eq!(saved_work.request, Some(request));
        assert_eq!(saved_work.owner, bound.acquire.owner);
        assert_eq!(saved_request.work, Some(work_id));
        assert_eq!(saved_request.owner, bound.acquire.owner);
        assert_eq!(saved_request.resource, bound.acquire.resource);
        assert!(saved_request.timed);
        assert_eq!(saved_request.submitted_at, arrival.at);

        let submitted = bound.finish_transit(&flow).unwrap();
        assert_eq!(submitted.work(), work_id);
        assert_eq!(submitted.request(), request);
    }

    #[test]
    fn runtime_rejected_start_at_u128_max_retries_once_at_same_due_and_priority() {
        let max = SimTime::from_ticks(u128::MAX);
        let (mut bound, mut flow) = bound_route_at(max, 1);
        let original = bound.start_transit(&mut flow).unwrap();
        let carrier = bound.carrier.unwrap();
        let rejected_start = flow.step().unwrap().expect("actual scheduled start event");
        assert_eq!(rejected_start.event, original);
        assert_eq!(rejected_start.at, max);
        assert_overflow_rejection(&rejected_start);
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &rejected_start),
            Ok(TransitObservation::Rejected)
        );
        let context = flow.work_context::<TransitContext>(carrier).unwrap();
        assert_eq!(context.phase(), TransitPhase::Ready);
        assert!(context.expects_event(original, max));

        let before_retry = flow.budget_snapshot().scheduler;
        let replacement = bound.retry_transit(&mut flow, &rejected_start).unwrap();
        assert_ne!(replacement, original);
        assert_eq!(
            flow.budget_snapshot().scheduler.scheduled_events,
            before_retry.scheduled_events + 1
        );
        assert!(flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .expects_event(replacement, max));
        let after_retry = flow.budget_snapshot().scheduler;
        assert_eq!(
            bound.retry_transit(&mut flow, &rejected_start),
            Err(BridgeError::InvalidDispatch)
        );
        assert_eq!(flow.budget_snapshot().scheduler, after_retry);
        assert_eq!(bound.pending_priority, Some(7));

        // Bracket the retry with same-time events: the replacement sorts after
        // priority 6 and before priority 8, proving the retained priority is 7.
        let before_priority = flow
            .schedule_domain(carrier, EventKind::custom(0xC20), max, 6)
            .unwrap();
        let after_priority = flow
            .schedule_domain(carrier, EventKind::custom(0xC20), max, 8)
            .unwrap();
        assert_eq!(flow.step().unwrap().unwrap().event, before_priority);
        let retried = flow.step().unwrap().expect("replacement start retry");
        assert_eq!(retried.event, replacement);
        assert_eq!(retried.at, max);
        assert_overflow_rejection(&retried);
        assert_eq!(flow.step().unwrap().unwrap().event, after_priority);
    }

    #[test]
    fn runtime_rejected_progress_at_u128_max_retries_once_and_preserves_progress() {
        let max = SimTime::from_ticks(u128::MAX);
        let start = SimTime::from_ticks(u128::MAX - 1);
        let (mut bound, mut flow) = bound_route_at(start, 2);
        let start_event = bound.start_transit(&mut flow).unwrap();
        let carrier = bound.carrier.unwrap();
        let accepted_start = flow.step().unwrap().expect("actual scheduled start event");
        assert_eq!(accepted_start.event, start_event);
        assert_eq!(accepted_start.at, start);
        assert!(matches!(
            accepted_start.callback_batches.as_slice(),
            [FlowBatchReceipt::Accepted(_)]
        ));
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &accepted_start),
            Ok(TransitObservation::Progress)
        );
        let progress_event = bound.pending_event.unwrap();

        let rejected_progress = flow
            .step()
            .unwrap()
            .expect("actual scheduled progress event");
        assert_eq!(rejected_progress.event, progress_event);
        assert_eq!(rejected_progress.at, max);
        assert_overflow_rejection(&rejected_progress);
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &rejected_progress),
            Ok(TransitObservation::Rejected)
        );
        let context = flow.work_context::<TransitContext>(carrier).unwrap();
        assert_eq!(context.phase(), TransitPhase::Moving);
        let retained = context.progress_at(start).unwrap();
        assert_eq!(retained.useful_elapsed, SimDuration::ZERO);
        assert_eq!(retained.remaining, SimDuration::from_ticks(2));
        assert!(context.expects_event(progress_event, max));

        let before_retry = flow.budget_snapshot().scheduler;
        let replacement = bound.retry_transit(&mut flow, &rejected_progress).unwrap();
        assert_ne!(replacement, progress_event);
        assert_eq!(
            flow.budget_snapshot().scheduler.scheduled_events,
            before_retry.scheduled_events + 1
        );
        assert!(flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .expects_event(replacement, max));
        let after_retry = flow.budget_snapshot().scheduler;
        assert_eq!(
            bound.retry_transit(&mut flow, &rejected_progress),
            Err(BridgeError::InvalidDispatch)
        );
        assert_eq!(flow.budget_snapshot().scheduler, after_retry);
        assert_eq!(bound.pending_priority, Some(7));

        let before_priority = flow
            .schedule_domain(carrier, EventKind::custom(0xC20), max, 6)
            .unwrap();
        let after_priority = flow
            .schedule_domain(carrier, EventKind::custom(0xC20), max, 8)
            .unwrap();
        assert_eq!(flow.step().unwrap().unwrap().event, before_priority);
        let retried = flow.step().unwrap().expect("replacement progress retry");
        assert_eq!(retried.event, replacement);
        assert_eq!(retried.at, max);
        assert_overflow_rejection(&retried);
        assert_eq!(flow.step().unwrap().unwrap().event, after_priority);
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

#[cfg(test)]
#[path = "../../../conformance/c20/transit_flow_c20.rs"]
mod transit_flow_c20;
