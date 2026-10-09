//! Shared agent behavior on the authoritative Flow runtime.
use super::spatial::{
    RouteCheckpointError, RoutePlan, RoutePlanImageLimitsV1, RoutePlanImageV1, TransitError,
    TransitGraphV1, TransitProgressState,
};
use kairo_ecs_des::{
    FlowAcquireCommand, FlowBatchReceipt, FlowCallbackCause, FlowCallbackCodeV1,
    FlowCallbackSnapshot, FlowCheckpointCodecs, FlowCheckpointError, FlowCheckpointRebindV1,
    FlowCommandSink, FlowCommandTicket, FlowDomainControl, FlowError, FlowOwnedCommand,
    FlowRuntime, FlowRuntimeIdentity, FlowWorldView, PreemptionStrategy, WorkId, WorkState,
};
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_types::{EntityId, EventId, EventKind, SimDuration, SimTime};
use std::cmp::min;
use std::error::Error;
use std::fmt::{Display, Formatter};

const TRANSIT_CONTEXT_SCHEMA_V1: u32 = 1;
const TRANSIT_CONTEXT_FIXED_IMAGE_BYTES: usize = 512;

/// Bounds for one native TransitContext image. Route limits are checked before
/// route payload cloning; `max_total_bytes` also includes this context envelope.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransitContextCheckpointLimitsV1 {
    pub max_segments: usize,
    pub max_graph_bytes: usize,
    pub max_mode_bytes: usize,
    pub max_total_bytes: usize,
}

impl TransitContextCheckpointLimitsV1 {
    pub const fn new(
        max_segments: usize,
        max_graph_bytes: usize,
        max_mode_bytes: usize,
        max_total_bytes: usize,
    ) -> Self {
        Self {
            max_segments,
            max_graph_bytes,
            max_mode_bytes,
            max_total_bytes,
        }
    }

    fn route_limits(self) -> RoutePlanImageLimitsV1 {
        RoutePlanImageLimitsV1::new(
            self.max_segments,
            self.max_graph_bytes,
            self.max_mode_bytes,
            self.max_total_bytes
                .saturating_sub(TRANSIT_CONTEXT_FIXED_IMAGE_BYTES),
        )
    }

    fn check_context_size(
        self,
        segments: usize,
        graph_bytes: usize,
        mode_bytes: usize,
    ) -> Result<(), TransitContextCheckpointError> {
        let total = segments
            .checked_mul(64)
            .and_then(|bytes| bytes.checked_add(graph_bytes))
            .and_then(|bytes| bytes.checked_add(mode_bytes))
            .and_then(|bytes| bytes.checked_add(TRANSIT_CONTEXT_FIXED_IMAGE_BYTES))
            .ok_or(TransitContextCheckpointError::LimitExceeded)?;
        if segments > self.max_segments
            || graph_bytes > self.max_graph_bytes
            || mode_bytes > self.max_mode_bytes
            || total > self.max_total_bytes
        {
            return Err(TransitContextCheckpointError::LimitExceeded);
        }
        Ok(())
    }
}

/// Errors from the experimental native TransitContext checkpoint seam.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransitContextCheckpointError {
    UnsupportedVersion(u32),
    LimitExceeded,
    LineageMismatch,
    InvalidState,
    InvalidReference,
    InvalidRoute,
    InvalidProgress,
    InvalidTicket,
}

impl Display for TransitContextCheckpointError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported transit context image version: {version}")
            }
            Self::LimitExceeded => f.write_str("transit context checkpoint limit exceeded"),
            Self::LineageMismatch => f.write_str("transit context belongs to another Flow runtime"),
            Self::InvalidState => f.write_str("invalid transit context state"),
            Self::InvalidReference => f.write_str("invalid transit context reference"),
            Self::InvalidRoute => f.write_str("invalid transit context route"),
            Self::InvalidProgress => f.write_str("invalid transit context progress"),
            Self::InvalidTicket => f.write_str("invalid transit context ticket"),
        }
    }
}

impl Error for TransitContextCheckpointError {}

/// Ticket purpose retained as native state for a staged callback command.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitCommandPurposeCheckpointV1 {
    Start { due_ticks: u128 },
    Progress { due_ticks: u128 },
    Arrival,
}

/// Native owned v1 checkpoint of one transit context. Opaque runtime identity
/// and pointer-bearing values are deliberately absent.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitContextCheckpointV1 {
    pub schema_version: u32,
    pub route: RoutePlanImageV1,
    pub segment_index: usize,
    pub elapsed_in_segment_ticks: u128,
    pub service_work: EntityId,
    pub acquire_resource: EntityId,
    pub acquire_owner: EntityId,
    pub acquire_work: Option<EntityId>,
    pub acquire_at_ticks: u128,
    pub acquire_priority_level: i32,
    pub acquire_deadline_ticks: Option<u128>,
    pub acquire_scheduler_priority: i32,
    pub acquire_timed: bool,
    pub acquire_can_preempt: bool,
    pub acquire_preemptible: Option<PreemptionStrategy>,
    pub carrier: Option<EntityId>,
    pub kind: Option<EventKind>,
    pub start_at_ticks: u128,
    pub phase: TransitPhase,
    pub paused_from: Option<TransitPhase>,
    pub last_advanced_at_ticks: u128,
    pub initial_start_pending: bool,
    pub expected_event: Option<(u64, u32)>,
    pub expected_due_ticks: Option<u128>,
    pub command_ticket: Option<(u64, usize, TransitCommandPurposeCheckpointV1)>,
    pub arrival_ticket: Option<(u64, usize)>,
    pub next_progress_ticket: Option<(u64, usize)>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlowAgentHandle {
    actor: EntityId,
    carrier: WorkId,
    kind: EventKind,
}
pub struct FlowAgentContext<'a, C> {
    pub agent: EntityId,
    pub event: &'a FlowCallbackSnapshot,
    pub state: &'a mut C,
    pub rng: &'a mut DeterministicStream,
    pub view: FlowWorldView<'a>,
    pub commands: &'a mut FlowCommandSink,
}
pub trait FlowAgentBehavior<C> {
    fn update(&mut self, context: FlowAgentContext<'_, C>);
}
struct FlowAgentState<C, B> {
    actor: EntityId,
    state: C,
    behavior: B,
    rng: DeterministicStream,
}
fn dispatch_flow_agent<'a, C, B: FlowAgentBehavior<C>>(
    row: &'a mut FlowAgentState<C, B>,
    event: &'a FlowCallbackSnapshot,
    view: FlowWorldView<'a>,
    commands: &'a mut FlowCommandSink,
) {
    let FlowAgentState {
        actor,
        state,
        behavior,
        rng,
    } = row;
    behavior.update(FlowAgentContext {
        agent: *actor,
        event,
        state,
        rng,
        view,
        commands,
    });
}
pub fn register_flow_agent_behavior<C: 'static, B: FlowAgentBehavior<C> + 'static>(
    flow: &mut FlowRuntime,
    registration: &str,
    kind: EventKind,
) -> Result<(), FlowError> {
    flow.ensure_pre_work_registration()?;
    flow.register_domain_view_hook::<FlowAgentState<C, B>>(
        registration,
        kind,
        dispatch_flow_agent::<C, B>,
    )
}
pub fn create_flow_agent<C: 'static, B: FlowAgentBehavior<C> + 'static>(
    flow: &mut FlowRuntime,
    actor: EntityId,
    registration: &str,
    kind: EventKind,
    run_seed: u64,
    state: C,
    behavior: B,
) -> Result<FlowAgentHandle, FlowError> {
    let row = FlowAgentState {
        actor,
        state,
        behavior,
        rng: DeterministicStream::from_entity(run_seed, actor),
    };
    let carrier = flow.create_actor_domain_context(actor, registration, kind, row)?;
    Ok(FlowAgentHandle {
        actor,
        carrier,
        kind,
    })
}
pub fn schedule_flow_agent_update(
    flow: &mut FlowRuntime,
    agent: FlowAgentHandle,
    at: SimTime,
    scheduler_priority: i32,
) -> Result<EventId, FlowError> {
    if flow.actor_domain_context(agent.actor)? != agent.carrier
        || flow.work(agent.carrier)?.owner != agent.actor
    {
        return Err(FlowError::InvalidWork);
    }
    flow.schedule_domain(agent.carrier, agent.kind, at, scheduler_priority)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitPhase {
    Ready,
    Moving,
    Paused,
    Arrived,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitProgress {
    pub segment_index: usize,
    pub useful_elapsed: SimDuration,
    pub remaining: SimDuration,
    pub phase: TransitPhase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TransitCommandPurpose {
    Start { due: SimTime },
    Progress { due: SimTime },
    Arrival,
}

#[derive(Clone)]
pub struct TransitContext {
    runtime: FlowRuntimeIdentity,
    route_plan: RoutePlan,
    progress: TransitProgressState,
    service_work: WorkId,
    acquire: FlowAcquireCommand,
    carrier: Option<WorkId>,
    kind: Option<EventKind>,
    start_at: SimTime,
    phase: TransitPhase,
    paused_from: Option<TransitPhase>,
    last_advanced_at: SimTime,
    initial_start_pending: bool,
    expected_event: Option<EventId>,
    expected_due: Option<SimTime>,
    command_ticket: Option<(FlowCommandTicket, TransitCommandPurpose)>,
    arrival_ticket: Option<FlowCommandTicket>,
    next_progress_ticket: Option<FlowCommandTicket>,
}

impl TransitContext {
    /// Capture all native mutable state after verifying that the context belongs
    /// to the supplied source runtime. This is an owner payload only: it does
    /// not assert a coherent outer Flow/bridge frontier.
    #[doc(hidden)]
    pub fn checkpoint_v1(
        &self,
        source_identity: &FlowRuntimeIdentity,
        limits: TransitContextCheckpointLimitsV1,
    ) -> Result<TransitContextCheckpointV1, TransitContextCheckpointError> {
        if &self.runtime != source_identity {
            return Err(TransitContextCheckpointError::LineageMismatch);
        }
        let (segment_index, elapsed) = self
            .progress
            .checkpoint_cursor()
            .map_err(|_| TransitContextCheckpointError::InvalidProgress)?;
        if !self.progress.uses_route(&self.route_plan) {
            return Err(TransitContextCheckpointError::InvalidRoute);
        }
        validate_transit_context_state(TransitStateFacts {
            phase: self.phase,
            paused_from: self.paused_from,
            initial_start_pending: self.initial_start_pending,
            acquire_timed: self.acquire.timed,
            acquire_at_ticks: self.acquire.at.ticks(),
            start_at_ticks: self.start_at.ticks(),
            last_advanced_at_ticks: self.last_advanced_at.ticks(),
            has_carrier: self.carrier.is_some(),
            has_kind: self.kind.is_some(),
            initial_cursor: self.progress.is_initial_cursor(),
            complete_cursor: self.progress.is_complete_cursor(),
        })?;
        limits.check_context_size(
            self.route_plan.segments().len(),
            self.route_plan.graph_canonical_bytes().len(),
            self.route_plan.profile().mode().as_str().len(),
        )?;
        let route = self
            .route_plan
            .checkpoint_image_v1(&limits.route_limits())
            .map_err(map_route_checkpoint_error)?;
        let ticket_parts = |ticket: FlowCommandTicket| ticket.checkpoint_parts();
        Ok(TransitContextCheckpointV1 {
            schema_version: TRANSIT_CONTEXT_SCHEMA_V1,
            route,
            segment_index,
            elapsed_in_segment_ticks: elapsed.ticks(),
            service_work: self.service_work.entity_id(),
            acquire_resource: self.acquire.resource.entity_id(),
            acquire_owner: self.acquire.owner,
            acquire_work: self.acquire.work.map(WorkId::entity_id),
            acquire_at_ticks: self.acquire.at.ticks(),
            acquire_priority_level: self.acquire.priority_level,
            acquire_deadline_ticks: self.acquire.deadline.map(SimTime::ticks),
            acquire_scheduler_priority: self.acquire.scheduler_priority,
            acquire_timed: self.acquire.timed,
            acquire_can_preempt: self.acquire.can_preempt,
            acquire_preemptible: self.acquire.preemptible,
            carrier: self.carrier.map(WorkId::entity_id),
            kind: self.kind,
            start_at_ticks: self.start_at.ticks(),
            phase: self.phase,
            paused_from: self.paused_from,
            last_advanced_at_ticks: self.last_advanced_at.ticks(),
            initial_start_pending: self.initial_start_pending,
            expected_event: self
                .expected_event
                .map(|event| (event.index, event.generation)),
            expected_due_ticks: self.expected_due.map(SimTime::ticks),
            command_ticket: self.command_ticket.map(|(ticket, purpose)| {
                let (batch, index) = ticket_parts(ticket);
                let purpose = match purpose {
                    TransitCommandPurpose::Start { due } => {
                        TransitCommandPurposeCheckpointV1::Start {
                            due_ticks: due.ticks(),
                        }
                    }
                    TransitCommandPurpose::Progress { due } => {
                        TransitCommandPurposeCheckpointV1::Progress {
                            due_ticks: due.ticks(),
                        }
                    }
                    TransitCommandPurpose::Arrival => TransitCommandPurposeCheckpointV1::Arrival,
                };
                (batch, index, purpose)
            }),
            arrival_ticket: self.arrival_ticket.map(ticket_parts),
            next_progress_ticket: self.next_progress_ticket.map(ticket_parts),
        })
    }
}

impl TransitContextCheckpointV1 {
    /// Restores this context against the caller-approved immutable graph and a
    /// validated read-only Flow ID/ticket rebinding view.
    #[doc(hidden)]
    pub fn restore(
        &self,
        graph: &TransitGraphV1,
        rebind: &FlowCheckpointRebindV1,
        limits: TransitContextCheckpointLimitsV1,
    ) -> Result<TransitContext, TransitContextCheckpointError> {
        if self.schema_version != TRANSIT_CONTEXT_SCHEMA_V1 {
            return Err(TransitContextCheckpointError::UnsupportedVersion(
                self.schema_version,
            ));
        }
        self.route
            .validate_limits(&limits.route_limits())
            .map_err(map_route_checkpoint_error)?;
        limits.check_context_size(
            self.route.segments.len(),
            self.route.graph_canonical_bytes.len(),
            self.route.movement_mode.len(),
        )?;
        let service_work = rebind
            .resolve_work(self.service_work)
            .map_err(|_| TransitContextCheckpointError::InvalidReference)?;
        let service_owner = rebind
            .resolve_work_owner(self.service_work)
            .map_err(|_| TransitContextCheckpointError::InvalidReference)?;
        if service_owner != self.acquire_owner {
            return Err(TransitContextCheckpointError::InvalidReference);
        }
        let acquire_work = self
            .acquire_work
            .map(|id| rebind.resolve_work(id))
            .transpose()
            .map_err(|_| TransitContextCheckpointError::InvalidReference)?;
        if acquire_work != Some(service_work) {
            return Err(TransitContextCheckpointError::InvalidReference);
        }
        let acquire = FlowAcquireCommand {
            resource: rebind
                .resolve_resource(self.acquire_resource)
                .map_err(|_| TransitContextCheckpointError::InvalidReference)?,
            owner: rebind
                .resolve_actor(self.acquire_owner)
                .map_err(|_| TransitContextCheckpointError::InvalidReference)?,
            work: acquire_work,
            at: SimTime::from_ticks(self.acquire_at_ticks),
            priority_level: self.acquire_priority_level,
            deadline: self.acquire_deadline_ticks.map(SimTime::from_ticks),
            scheduler_priority: self.acquire_scheduler_priority,
            timed: self.acquire_timed,
            can_preempt: self.acquire_can_preempt,
            preemptible: self.acquire_preemptible,
        };
        let carrier = self
            .carrier
            .map(|id| rebind.resolve_work(id))
            .transpose()
            .map_err(|_| TransitContextCheckpointError::InvalidReference)?;
        if let Some(carrier_work) = carrier {
            let (_, _, domain_kind) = rebind
                .resolve_work_binding(carrier_work.entity_id())
                .map_err(|_| TransitContextCheckpointError::InvalidReference)?;
            if domain_kind != self.kind {
                return Err(TransitContextCheckpointError::InvalidReference);
            }
        }
        let expected_event = self
            .expected_event
            .map(|(index, generation)| rebind.resolve_issued_event(EventId::new(index, generation)))
            .transpose()
            .map_err(|_| TransitContextCheckpointError::InvalidReference)?;
        let restore_ticket = |parts: (u64, usize)| {
            rebind
                .resolve_ticket(parts.0, parts.1)
                .map_err(|_| TransitContextCheckpointError::InvalidTicket)
        };
        let command_ticket = self
            .command_ticket
            .map(|(batch, index, purpose)| {
                let purpose = match purpose {
                    TransitCommandPurposeCheckpointV1::Start { due_ticks } => {
                        TransitCommandPurpose::Start {
                            due: SimTime::from_ticks(due_ticks),
                        }
                    }
                    TransitCommandPurposeCheckpointV1::Progress { due_ticks } => {
                        TransitCommandPurpose::Progress {
                            due: SimTime::from_ticks(due_ticks),
                        }
                    }
                    TransitCommandPurposeCheckpointV1::Arrival => TransitCommandPurpose::Arrival,
                };
                restore_ticket((batch, index)).map(|ticket| (ticket, purpose))
            })
            .transpose()?;
        let arrival_ticket = self.arrival_ticket.map(restore_ticket).transpose()?;
        let next_progress_ticket = self.next_progress_ticket.map(restore_ticket).transpose()?;
        let route_plan = RoutePlan::restore_image_v1(&self.route, graph, &limits.route_limits())
            .map_err(map_route_checkpoint_error)?;
        let progress = TransitProgressState::restore_at_cursor(
            route_plan.clone(),
            self.segment_index,
            SimDuration::from_ticks(self.elapsed_in_segment_ticks),
        )
        .map_err(map_route_checkpoint_error)?;
        validate_transit_context_state(TransitStateFacts {
            phase: self.phase,
            paused_from: self.paused_from,
            initial_start_pending: self.initial_start_pending,
            acquire_timed: self.acquire_timed,
            acquire_at_ticks: self.acquire_at_ticks,
            start_at_ticks: self.start_at_ticks,
            last_advanced_at_ticks: self.last_advanced_at_ticks,
            has_carrier: self.carrier.is_some(),
            has_kind: self.kind.is_some(),
            initial_cursor: progress.is_initial_cursor(),
            complete_cursor: progress.is_complete_cursor(),
        })?;
        Ok(TransitContext {
            runtime: rebind.identity().clone(),
            route_plan,
            progress,
            service_work,
            acquire,
            carrier,
            kind: self.kind,
            start_at: SimTime::from_ticks(self.start_at_ticks),
            phase: self.phase,
            paused_from: self.paused_from,
            last_advanced_at: SimTime::from_ticks(self.last_advanced_at_ticks),
            initial_start_pending: self.initial_start_pending,
            expected_event,
            expected_due: self.expected_due_ticks.map(SimTime::from_ticks),
            command_ticket,
            arrival_ticket,
            next_progress_ticket,
        })
    }

    /// Restore this owner payload for its actual dense WorkContext row. The
    /// row is supplied by the trusted Flow decoder, never by the image.
    #[doc(hidden)]
    pub fn restore_for_owner(
        &self,
        graph: &TransitGraphV1,
        rebind: &FlowCheckpointRebindV1,
        row_owner: EntityId,
        limits: TransitContextCheckpointLimitsV1,
    ) -> Result<TransitContext, TransitContextCheckpointError> {
        let (_, _, row_domain_kind) = rebind
            .resolve_work_binding(row_owner)
            .map_err(|_| TransitContextCheckpointError::InvalidReference)?;
        if row_domain_kind.is_none() {
            return Err(TransitContextCheckpointError::InvalidReference);
        }
        match self.carrier {
            Some(carrier) => {
                if carrier != row_owner || self.kind != row_domain_kind {
                    return Err(TransitContextCheckpointError::InvalidReference);
                }
            }
            None => {
                if self.phase != TransitPhase::Ready
                    || self.paused_from.is_some()
                    || self.kind.is_some()
                {
                    return Err(TransitContextCheckpointError::InvalidState);
                }
            }
        }
        self.restore(graph, rebind, limits)
    }
}

struct TransitStateFacts {
    phase: TransitPhase,
    paused_from: Option<TransitPhase>,
    initial_start_pending: bool,
    acquire_timed: bool,
    acquire_at_ticks: u128,
    start_at_ticks: u128,
    last_advanced_at_ticks: u128,
    has_carrier: bool,
    has_kind: bool,
    initial_cursor: bool,
    complete_cursor: bool,
}

fn validate_transit_context_state(
    facts: TransitStateFacts,
) -> Result<(), TransitContextCheckpointError> {
    let TransitStateFacts {
        phase,
        paused_from,
        initial_start_pending,
        acquire_timed,
        acquire_at_ticks,
        start_at_ticks,
        last_advanced_at_ticks,
        has_carrier,
        has_kind,
        initial_cursor,
        complete_cursor,
    } = facts;
    let paused_ready = phase == TransitPhase::Paused && paused_from == Some(TransitPhase::Ready);
    let paused_moving = phase == TransitPhase::Paused && paused_from == Some(TransitPhase::Moving);
    if (phase == TransitPhase::Paused) != paused_from.is_some()
        || (paused_from.is_some()
            && !matches!(
                paused_from,
                Some(TransitPhase::Ready | TransitPhase::Moving)
            ))
        || !acquire_timed
        || acquire_at_ticks != start_at_ticks
        || has_carrier != has_kind
        || (!has_carrier && phase != TransitPhase::Ready)
        || (matches!(phase, TransitPhase::Moving | TransitPhase::Arrived) && !has_carrier)
        || (initial_start_pending
            && (matches!(phase, TransitPhase::Moving | TransitPhase::Arrived) || paused_moving))
        || ((phase == TransitPhase::Ready || paused_ready) && !initial_cursor)
        || (phase == TransitPhase::Arrived && !complete_cursor)
        || (phase == TransitPhase::Moving && complete_cursor)
        || ((phase == TransitPhase::Ready || paused_ready)
            && last_advanced_at_ticks != start_at_ticks)
        || (!(phase == TransitPhase::Ready || paused_ready)
            && last_advanced_at_ticks < start_at_ticks)
    {
        return Err(TransitContextCheckpointError::InvalidState);
    }
    Ok(())
}

fn map_route_checkpoint_error(error: RouteCheckpointError) -> TransitContextCheckpointError {
    match error {
        RouteCheckpointError::UnsupportedVersion => TransitContextCheckpointError::InvalidRoute,
        RouteCheckpointError::LimitExceeded => TransitContextCheckpointError::LimitExceeded,
        RouteCheckpointError::InvalidProgress => TransitContextCheckpointError::InvalidProgress,
        RouteCheckpointError::InvalidPlan | RouteCheckpointError::InvalidGraph => {
            TransitContextCheckpointError::InvalidRoute
        }
    }
}

impl TransitContext {
    /// Atomically retry a structurally matching rejected transit event.
    ///
    /// The calibration adapter must establish provenance for the returned dispatch.
    #[doc(hidden)]
    pub fn validate_retry_dispatch(
        flow: &FlowRuntime,
        carrier: WorkId,
        rejected: &kairo_ecs_des::FlowDispatch,
    ) -> Result<(), FlowError> {
        let context = flow.work_context::<TransitContext>(carrier)?;
        if context.runtime != flow.identity()
            || !matches!(context.phase, TransitPhase::Ready | TransitPhase::Moving)
            || context.expected_event != Some(rejected.event)
            || context.expected_due != Some(rejected.at)
            || rejected.at != flow.now()
            || !matches!(
                rejected.callback_batches.as_slice(),
                [FlowBatchReceipt::Rejected(_)]
            )
        {
            return Err(FlowError::InvalidWork);
        }
        Ok(())
    }

    #[doc(hidden)]
    pub fn schedule_transit_retry(
        flow: &mut FlowRuntime,
        carrier: WorkId,
        kind: EventKind,
        rejected: &kairo_ecs_des::FlowDispatch,
        priority: i32,
    ) -> Result<EventId, FlowError> {
        Self::validate_retry_dispatch(flow, carrier, rejected)?;
        flow.schedule_domain_and_bind::<TransitContext>(
            carrier,
            kind,
            rejected.at,
            priority,
            bind_retried_transit_event,
        )
    }

    fn bind_initial_start_event(&mut self, event: EventId) {
        self.expected_event = Some(event);
        self.expected_due = Some(self.start_at);
    }

    pub fn new(
        flow: &FlowRuntime,
        route: RoutePlan,
        acquire: FlowAcquireCommand,
        start: SimTime,
    ) -> Result<Self, TransitError> {
        let Some(work) = acquire.work else {
            return Err(TransitError::InvalidProgress);
        };
        let spec = flow.work(work).map_err(|_| TransitError::InvalidProgress)?;
        let progress = flow
            .work_progress(work)
            .map_err(|_| TransitError::InvalidProgress)?;
        flow.resource(acquire.resource)
            .map_err(|_| TransitError::InvalidProgress)?;
        if start < flow.now()
            || acquire.at != start
            || !acquire.timed
            || acquire.owner != spec.owner
            || spec.request.is_some()
            || progress.state != WorkState::Pending
        {
            return Err(TransitError::InvalidProgress);
        }
        let progress = TransitProgressState::new(route.clone())?;
        Ok(Self {
            runtime: flow.identity(),
            route_plan: route,
            progress,
            service_work: work,
            acquire,
            carrier: None,
            kind: None,
            start_at: start,
            phase: TransitPhase::Ready,
            paused_from: None,
            last_advanced_at: start,
            initial_start_pending: true,
            expected_event: None,
            expected_due: None,
            command_ticket: None,
            arrival_ticket: None,
            next_progress_ticket: None,
        })
    }

    pub fn service_work(&self) -> WorkId {
        self.service_work
    }

    /// Returns the immutable route actually retained by this transit carrier.
    ///
    /// This experimental readback is for the private calibration adapter; it
    /// does not establish a stable API or checkpoint compatibility promise.
    #[doc(hidden)]
    pub fn route_plan(&self) -> &RoutePlan {
        &self.route_plan
    }

    pub fn phase(&self) -> TransitPhase {
        self.phase
    }

    #[doc(hidden)]
    pub fn expects_event(&self, event: EventId, at: SimTime) -> bool {
        self.expected_event == Some(event) && self.expected_due == Some(at)
    }

    pub fn arrival_ticket(&self) -> Option<FlowCommandTicket> {
        self.arrival_ticket
    }

    pub fn next_progress_ticket(&self) -> Option<FlowCommandTicket> {
        self.next_progress_ticket
    }

    pub fn progress_at(&self, at: SimTime) -> Result<TransitProgress, TransitError> {
        let mut state = self.progress.clone();
        if self.phase == TransitPhase::Moving {
            let elapsed = at
                .duration_since(self.last_advanced_at)
                .ok_or(TransitError::InvalidProgress)?;
            state.advance(min(elapsed, state.remaining()?))?;
        } else if at < self.start_at && self.phase == TransitPhase::Ready {
            return Err(TransitError::InvalidProgress);
        }
        Ok(TransitProgress {
            segment_index: state.segment_index(),
            useful_elapsed: state.useful_elapsed()?,
            remaining: state.remaining()?,
            phase: self.phase,
        })
    }

    pub fn plan<'a>(
        current: &'a Self,
        snapshot: &'a FlowCallbackSnapshot,
        view: FlowWorldView<'a>,
        sink: &'a mut FlowCommandSink,
    ) -> Result<Self, FlowError> {
        let mut next = current.clone();
        let at = view.now();
        next.carrier = Some(snapshot.work);
        next.kind = match snapshot.cause {
            FlowCallbackCause::Domain { kind } | FlowCallbackCause::DomainControl { kind, .. } => {
                Some(kind)
            }
            FlowCallbackCause::Work { .. } => return Err(FlowError::InvalidWork),
        };
        match snapshot.cause {
            FlowCallbackCause::Domain { .. } => next.plan_domain(snapshot, at, sink)?,
            FlowCallbackCause::DomainControl { action, .. } => {
                next.plan_control(action, at, sink)?
            }
            FlowCallbackCause::Work { .. } => return Err(FlowError::InvalidWork),
        }
        Ok(next)
    }

    fn plan_domain(
        &mut self,
        snapshot: &FlowCallbackSnapshot,
        at: SimTime,
        sink: &mut FlowCommandSink,
    ) -> Result<(), FlowError> {
        if self.phase == TransitPhase::Paused {
            if self.initial_start_pending
                && self.expected_event == Some(snapshot.delivery.id)
                && self.expected_due == Some(at)
            {
                self.initial_start_pending = false;
            }
            if self.expected_event == Some(snapshot.delivery.id) && self.expected_due == Some(at) {
                self.expected_event = None;
                self.expected_due = None;
                self.next_progress_ticket = None;
            }
            return Ok(());
        }
        if self.phase == TransitPhase::Ready {
            if self.expected_event != Some(snapshot.delivery.id) || self.expected_due != Some(at) {
                return Ok(());
            }
            self.initial_start_pending = false;
            self.expected_event = None;
            self.expected_due = None;
            self.phase = TransitPhase::Moving;
            self.last_advanced_at = at;
            return self.schedule_next_or_arrive(at, sink);
        }
        if self.phase != TransitPhase::Moving
            || self.expected_event != Some(snapshot.delivery.id)
            || self.expected_due != Some(at)
        {
            return Ok(());
        }
        let elapsed = at
            .duration_since(self.last_advanced_at)
            .ok_or(FlowError::InvalidWork)?;
        self.progress
            .advance(min(
                elapsed,
                self.progress
                    .remaining()
                    .map_err(|_| FlowError::InvalidWork)?,
            ))
            .map_err(|_| FlowError::InvalidWork)?;
        self.last_advanced_at = at;
        self.expected_event = None;
        self.expected_due = None;
        self.next_progress_ticket = None;
        self.schedule_next_or_arrive(at, sink)
    }

    fn plan_control(
        &mut self,
        action: FlowDomainControl,
        at: SimTime,
        sink: &mut FlowCommandSink,
    ) -> Result<(), FlowError> {
        match (action, self.phase) {
            (FlowDomainControl::Pause, TransitPhase::Ready) if at <= self.start_at => {
                self.paused_from = Some(TransitPhase::Ready);
                self.phase = TransitPhase::Paused;
                Ok(())
            }
            (FlowDomainControl::Pause, TransitPhase::Moving) => {
                let elapsed = at
                    .duration_since(self.last_advanced_at)
                    .ok_or(FlowError::InvalidWork)?;
                self.progress
                    .advance(min(
                        elapsed,
                        self.progress
                            .remaining()
                            .map_err(|_| FlowError::InvalidWork)?,
                    ))
                    .map_err(|_| FlowError::InvalidWork)?;
                self.last_advanced_at = at;
                self.paused_from = Some(TransitPhase::Moving);
                self.phase = TransitPhase::Paused;
                Ok(())
            }
            (FlowDomainControl::Resume, TransitPhase::Paused) => {
                let was = self.paused_from.take().ok_or(FlowError::InvalidWork)?;
                self.phase = was;
                match was {
                    TransitPhase::Ready => {
                        let due = if self.initial_start_pending {
                            self.start_at
                        } else {
                            max_time(at, self.start_at)
                        };
                        if self.initial_start_pending
                            || (self.expected_event.is_some() && self.expected_due == Some(due))
                        {
                            Ok(())
                        } else {
                            self.emit_domain(due, TransitCommandPurpose::Start { due }, sink)
                        }
                    }
                    TransitPhase::Moving => {
                        if self
                            .progress
                            .remaining()
                            .map_err(|_| FlowError::InvalidWork)?
                            == SimDuration::ZERO
                        {
                            return self.emit_arrival(at, sink);
                        }
                        let remaining = self
                            .progress
                            .current_segment_remaining()
                            .map_err(|_| FlowError::InvalidWork)?;
                        let due = at
                            .checked_add(remaining)
                            .ok_or(FlowError::CounterOverflow)?;
                        if self.expected_event.is_some() && self.expected_due == Some(due) {
                            self.last_advanced_at = at;
                            return Ok(());
                        }
                        self.last_advanced_at = at;
                        self.emit_domain(due, TransitCommandPurpose::Progress { due }, sink)
                    }
                    _ => Err(FlowError::InvalidWork),
                }
            }
            _ => Err(FlowError::InvalidState),
        }
    }

    fn schedule_next_or_arrive(
        &mut self,
        at: SimTime,
        sink: &mut FlowCommandSink,
    ) -> Result<(), FlowError> {
        if self
            .progress
            .remaining()
            .map_err(|_| FlowError::InvalidWork)?
            == SimDuration::ZERO
        {
            return self.emit_arrival(at, sink);
        }
        let remaining = self
            .progress
            .current_segment_remaining()
            .map_err(|_| FlowError::InvalidWork)?;
        let due = at
            .checked_add(remaining)
            .ok_or(FlowError::CounterOverflow)?;
        self.last_advanced_at = at;
        self.emit_domain(due, TransitCommandPurpose::Progress { due }, sink)
    }

    fn emit_domain(
        &mut self,
        due: SimTime,
        purpose: TransitCommandPurpose,
        sink: &mut FlowCommandSink,
    ) -> Result<(), FlowError> {
        let ticket = sink.emit(FlowOwnedCommand::Domain {
            work: self.carrier.ok_or(FlowError::InvalidWork)?,
            kind: self.kind.ok_or(FlowError::InvalidWork)?,
            at: due,
            scheduler_priority: self.acquire.scheduler_priority,
        })?;
        self.command_ticket = Some((ticket, purpose));
        self.expected_event = None;
        self.expected_due = Some(due);
        if matches!(purpose, TransitCommandPurpose::Progress { .. }) {
            self.next_progress_ticket = Some(ticket);
        }
        Ok(())
    }

    fn emit_arrival(&mut self, at: SimTime, sink: &mut FlowCommandSink) -> Result<(), FlowError> {
        let mut acquire = self.acquire.clone();
        acquire.at = at;
        let ticket = sink.emit(FlowOwnedCommand::Acquire(acquire))?;
        self.command_ticket = Some((ticket, TransitCommandPurpose::Arrival));
        self.arrival_ticket = Some(ticket);
        self.phase = TransitPhase::Arrived;
        Ok(())
    }

    fn apply_receipt(&mut self, receipt: &FlowBatchReceipt) {
        let Some((ticket, purpose)) = self.command_ticket.take() else {
            return;
        };
        let FlowBatchReceipt::Accepted(admissions) = receipt else {
            return;
        };
        let Some(admission) = admissions.iter().find(|row| row.ticket == ticket) else {
            return;
        };
        match purpose {
            TransitCommandPurpose::Start { due } => {
                self.expected_event = Some(admission.event);
                self.expected_due = Some(due);
            }
            TransitCommandPurpose::Progress { due } => {
                self.expected_event = Some(admission.event);
                self.expected_due = Some(due);
            }
            TransitCommandPurpose::Arrival => self.expected_due = None,
        }
    }
}

fn max_time(left: SimTime, right: SimTime) -> SimTime {
    if left < right {
        right
    } else {
        left
    }
}

fn accept_transit_context(context: &mut TransitContext, receipt: &FlowBatchReceipt) {
    context.apply_receipt(receipt);
}

pub fn register_transit_context(
    flow: &mut FlowRuntime,
    registration: &str,
    kind: EventKind,
) -> Result<(), FlowError> {
    flow.register_domain_plan_hook_with_receipt(
        registration,
        kind,
        TransitContext::plan,
        accept_transit_context,
    )
}

/// Register this trusted transit planner in the native Flow codec manifest.
/// The supplied callback IDs are caller-owned compatibility declarations.
#[doc(hidden)]
pub fn register_transit_context_checkpoint_domain(
    codecs: &mut FlowCheckpointCodecs,
    registration: &str,
    kind: EventKind,
    planner_id: FlowCallbackCodeV1,
    receipt_id: FlowCallbackCodeV1,
) -> Result<(), FlowCheckpointError> {
    codecs.register_domain_plan_hook_with_receipt::<TransitContext>(
        registration,
        kind,
        TransitContext::plan,
        accept_transit_context,
        planner_id,
        receipt_id,
    )
}

#[cfg(feature = "test-support")]
thread_local! {
    static REJECTED_TRANSIT_PLAN_FOR_TEST: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(feature = "test-support")]
fn reject_first_transit_plan_for_test<'a>(
    current: &'a TransitContext,
    snapshot: &'a FlowCallbackSnapshot,
    view: FlowWorldView<'a>,
    sink: &'a mut FlowCommandSink,
) -> Result<TransitContext, FlowError> {
    if REJECTED_TRANSIT_PLAN_FOR_TEST.with(|rejected| !rejected.replace(true)) {
        return Err(FlowError::InvalidState);
    }
    TransitContext::plan(current, snapshot, view, sink)
}

/// Test-only registration that rejects the first actual transit planner call on this thread.
///
/// This helper is available only with the non-default `test-support` feature.
#[cfg(feature = "test-support")]
#[doc(hidden)]
pub fn register_transit_context_reject_first_for_test(
    flow: &mut FlowRuntime,
    registration: &str,
    kind: EventKind,
) -> Result<(), FlowError> {
    flow.register_domain_plan_hook_with_receipt(
        registration,
        kind,
        reject_first_transit_plan_for_test,
        accept_transit_context,
    )
}

pub fn schedule_transit_control(
    flow: &mut FlowRuntime,
    carrier: WorkId,
    kind: EventKind,
    action: FlowDomainControl,
    at: SimTime,
    priority: i32,
) -> Result<EventId, FlowError> {
    let context = flow.work_context::<TransitContext>(carrier)?;
    if context.runtime != flow.identity() || context.phase == TransitPhase::Arrived {
        return Err(FlowError::InvalidWork);
    }
    flow.schedule_domain_control(carrier, kind, action, at, priority)
}

pub fn schedule_transit_start(
    flow: &mut FlowRuntime,
    carrier: WorkId,
    kind: EventKind,
    at: SimTime,
    priority: i32,
) -> Result<EventId, FlowError> {
    let context = flow.work_context::<TransitContext>(carrier)?;
    if context.runtime != flow.identity()
        || context.phase != TransitPhase::Ready
        || context.expected_event.is_some()
        || at != context.start_at
        || at < flow.now()
    {
        return Err(FlowError::InvalidWork);
    }
    flow.schedule_domain_and_bind::<TransitContext>(
        carrier,
        kind,
        at,
        priority,
        TransitContext::bind_initial_start_event,
    )
}

fn bind_retried_transit_event(context: &mut TransitContext, event: EventId) {
    context.expected_event = Some(event);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial::{
        EdgeId, MovementModeId, MovementProfile, NodeId, TransitEdge, TransitGraphV1,
    };
    use kairo_ecs_des::{FlowConfig, RequestState, ResourceId};
    use std::cell::Cell;
    use std::num::NonZeroU64;

    thread_local! {
        static PLANNER_CALLS: Cell<usize> = const { Cell::new(0) };
    }

    fn reject_start_and_first_progress_once(
        current: &TransitContext,
        snapshot: &FlowCallbackSnapshot,
        view: FlowWorldView<'_>,
        sink: &mut FlowCommandSink,
    ) -> Result<TransitContext, FlowError> {
        let call = PLANNER_CALLS.with(|calls| {
            let call = calls.get();
            calls.set(call + 1);
            call
        });
        if matches!(call, 0 | 2) {
            return Err(FlowError::InvalidState);
        }
        TransitContext::plan(current, snapshot, view, sink)
    }
    struct Draw {
        calls: u32,
    }
    impl FlowAgentBehavior<Vec<u64>> for Draw {
        fn update(&mut self, c: FlowAgentContext<'_, Vec<u64>>) {
            self.calls += 1;
            c.state.push(c.rng.next_u64());
        }
    }
    #[test]
    fn preconsume_budget_preserves_actual_private_stream_state_and_behavior() {
        let mut f = FlowRuntime::with_config(FlowConfig {
            max_same_tick_flow_transitions: NonZeroU64::new(1).unwrap(),
        });
        let kind = EventKind::custom(9010);
        register_flow_agent_behavior::<Vec<u64>, Draw>(&mut f, "draw", kind).unwrap();
        let actor = f.spawn_actor().unwrap();
        let agent = create_flow_agent(
            &mut f,
            actor,
            "draw",
            kind,
            17,
            Vec::<u64>::new(),
            Draw { calls: 0 },
        )
        .unwrap();
        let initial = f
            .work_context::<FlowAgentState<Vec<u64>, Draw>>(agent.carrier)
            .unwrap();
        assert_eq!(
            initial.rng.clone().into_inner(),
            DeterministicStream::from_entity(17, actor).into_inner()
        );
        assert!(initial.state.is_empty());
        schedule_flow_agent_update(&mut f, agent, SimTime::from_ticks(2), 0).unwrap();
        let pending = schedule_flow_agent_update(&mut f, agent, SimTime::from_ticks(2), 0).unwrap();
        assert!(f.step().unwrap().unwrap().error.is_none());
        let before = f
            .work_context::<FlowAgentState<Vec<u64>, Draw>>(agent.carrier)
            .unwrap();
        let state = before.state.clone();
        let raw = before.rng.clone().into_inner();
        let calls = before.behavior.calls;
        let stats = f.budget_snapshot().scheduler;
        for _ in 0..2 {
            assert_eq!(
                f.step().unwrap_err(),
                FlowError::SameTickBudgetExceeded {
                    at_ticks: 2,
                    limit: 1
                }
            );
            let after = f
                .work_context::<FlowAgentState<Vec<u64>, Draw>>(agent.carrier)
                .unwrap();
            assert_eq!(after.state, state);
            assert_eq!(after.rng.clone().into_inner(), raw);
            assert_eq!(after.behavior.calls, calls);
            assert_eq!(f.budget_snapshot().scheduler, stats);
            assert_eq!(f.budget_snapshot().halted.unwrap().pending.id, pending);
        }
        assert_eq!(calls, 1);
        assert_eq!(state.len(), 1);
    }

    fn test_route() -> RoutePlan {
        let nodes = (1..=3).map(NodeId::new).collect::<Vec<_>>();
        let mode = MovementModeId::new("walk").unwrap();
        let edges = vec![
            TransitEdge {
                id: EdgeId::new(1),
                from: nodes[0],
                to: nodes[1],
                length_mm: 1,
                allowed_modes: vec![mode.clone()],
            },
            TransitEdge {
                id: EdgeId::new(2),
                from: nodes[1],
                to: nodes[2],
                length_mm: 1,
                allowed_modes: vec![mode],
            },
        ];
        TransitGraphV1::new(1, nodes.clone(), edges)
            .unwrap()
            .route(
                nodes[0],
                nodes[2],
                &MovementProfile::new("walk", 1).unwrap(),
                1,
            )
            .unwrap()
    }

    fn zero_route() -> RoutePlan {
        let origin = NodeId::new(1);
        TransitGraphV1::new(1, vec![origin], vec![])
            .unwrap()
            .route(origin, origin, &MovementProfile::new("walk", 1).unwrap(), 1)
            .unwrap()
    }

    fn flow_acquire(resource: ResourceId, owner: EntityId, work: WorkId) -> FlowAcquireCommand {
        FlowAcquireCommand {
            resource,
            owner,
            work: Some(work),
            at: SimTime::from_ticks(5),
            priority_level: 0,
            deadline: None,
            scheduler_priority: 0,
            timed: true,
            can_preempt: false,
            preemptible: None,
        }
    }

    #[test]
    fn actual_flow_pause_resume_retains_route_and_submits_one_arrival_acquire() {
        const TRANSIT_KIND: EventKind = EventKind::custom(9410);
        let mut flow = FlowRuntime::new();
        register_transit_context(&mut flow, "transit-plan", TRANSIT_KIND).unwrap();
        let owner = flow.spawn_actor().unwrap();
        let carrier_actor = flow.spawn_actor().unwrap();
        let resource = flow.create_resource(1).unwrap();
        let service = flow
            .create_work(owner, SimDuration::from_ticks(4), "service", ())
            .unwrap();
        let acquire = flow_acquire(resource, owner, service);
        let context =
            TransitContext::new(&flow, test_route(), acquire, SimTime::from_ticks(5)).unwrap();
        let carrier = flow
            .create_actor_domain_context(carrier_actor, "transit-plan", TRANSIT_KIND, context)
            .unwrap();
        let mut foreign = FlowRuntime::new();
        let foreign_before = foreign.budget_snapshot();
        assert_eq!(
            schedule_transit_control(
                &mut foreign,
                carrier,
                TRANSIT_KIND,
                FlowDomainControl::Pause,
                SimTime::from_ticks(2),
                0,
            ),
            Err(FlowError::InvalidWork)
        );
        assert_eq!(foreign.budget_snapshot(), foreign_before);
        let retained_start =
            schedule_transit_start(&mut flow, carrier, TRANSIT_KIND, SimTime::from_ticks(5), 0)
                .unwrap();
        let before_duplicate = flow.budget_snapshot();
        let before_duplicate_context = flow.work_context::<TransitContext>(carrier).unwrap();
        assert_eq!(
            before_duplicate_context.expected_event,
            Some(retained_start)
        );
        assert_eq!(
            schedule_transit_start(&mut flow, carrier, TRANSIT_KIND, SimTime::from_ticks(5), 1),
            Err(FlowError::InvalidWork)
        );
        assert_eq!(flow.budget_snapshot(), before_duplicate);
        let after_duplicate_context = flow.work_context::<TransitContext>(carrier).unwrap();
        assert_eq!(after_duplicate_context.expected_event, Some(retained_start));
        assert_eq!(
            after_duplicate_context.expected_due,
            Some(SimTime::from_ticks(5))
        );
        let alien_start = flow
            .schedule_domain(carrier, TRANSIT_KIND, SimTime::from_ticks(5), -1)
            .unwrap();
        assert_ne!(retained_start, alien_start);

        schedule_transit_control(
            &mut flow,
            carrier,
            TRANSIT_KIND,
            FlowDomainControl::Pause,
            SimTime::from_ticks(2),
            0,
        )
        .unwrap();
        schedule_transit_control(
            &mut flow,
            carrier,
            TRANSIT_KIND,
            FlowDomainControl::Pause,
            SimTime::from_ticks(3),
            0,
        )
        .unwrap();
        let paused_ready = flow.step().unwrap().unwrap();
        assert!(paused_ready.error.is_none());
        let paused = flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .progress_at(SimTime::from_ticks(2))
            .unwrap();
        assert_eq!(paused.phase, TransitPhase::Paused);
        assert_eq!(paused.useful_elapsed, SimDuration::ZERO);
        assert_eq!(paused.remaining, SimDuration::from_ticks(2));
        let before_repeated_pause = paused.clone();
        let dispatches_before = flow.budget_snapshot().scheduler.dispatched_events;
        let repeated_pause = flow.step().unwrap().unwrap();
        assert_eq!(repeated_pause.error, Some(FlowError::InvalidState));
        assert!(
            matches!(repeated_pause.callback_batches.as_slice(), [FlowBatchReceipt::Rejected(row)] if row.error == FlowError::InvalidState && row.failed_ticket.is_none())
        );
        assert_eq!(
            flow.budget_snapshot().scheduler.dispatched_events,
            dispatches_before + 1
        );
        let after_repeated_pause = flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .progress_at(SimTime::from_ticks(3))
            .unwrap();
        assert_eq!(after_repeated_pause, before_repeated_pause);

        let alien_while_paused = flow.step().unwrap().unwrap();
        assert_eq!(alien_while_paused.event, alien_start);
        assert_eq!(alien_while_paused.at, SimTime::from_ticks(5));
        assert!(alien_while_paused.error.is_none());
        let paused_after_alien = flow.work_context::<TransitContext>(carrier).unwrap();
        assert_eq!(paused_after_alien.phase(), TransitPhase::Paused);
        assert_eq!(
            paused_after_alien
                .progress_at(SimTime::from_ticks(5))
                .unwrap()
                .useful_elapsed,
            SimDuration::ZERO
        );

        let stale_start = flow.step().unwrap().unwrap();
        assert_eq!(stale_start.event, retained_start);
        assert_eq!(stale_start.at, SimTime::from_ticks(5));
        assert!(stale_start.error.is_none());
        assert!(
            matches!(stale_start.callback_batches.as_slice(), [FlowBatchReceipt::Accepted(rows)] if rows.is_empty())
        );

        schedule_transit_control(
            &mut flow,
            carrier,
            TRANSIT_KIND,
            FlowDomainControl::Resume,
            SimTime::from_ticks(6),
            0,
        )
        .unwrap();
        let resume_ready = flow.step().unwrap().unwrap();
        assert!(resume_ready.error.is_none());
        let FlowBatchReceipt::Accepted(resume_rows) = &resume_ready.callback_batches[0] else {
            panic!(
                "Resume must plan a new start after the original start was consumed while paused"
            );
        };
        assert_eq!(resume_rows.len(), 1);
        assert_eq!(resume_rows[0].request, None);

        let start = flow.step().unwrap().unwrap();
        assert_eq!(start.at, SimTime::from_ticks(6));
        assert!(start.error.is_none());
        assert_eq!(
            flow.work_context::<TransitContext>(carrier)
                .unwrap()
                .phase(),
            TransitPhase::Moving
        );

        schedule_transit_control(
            &mut flow,
            carrier,
            TRANSIT_KIND,
            FlowDomainControl::Pause,
            SimTime::from_ticks(7),
            -1,
        )
        .unwrap();
        let paused_moving = flow.step().unwrap().unwrap();
        assert_eq!(paused_moving.at, SimTime::from_ticks(7));
        assert!(paused_moving.error.is_none());
        let after_pause = flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .progress_at(SimTime::from_ticks(7))
            .unwrap();
        assert_eq!(after_pause.phase, TransitPhase::Paused);
        assert_eq!(after_pause.segment_index, 1);
        assert_eq!(after_pause.useful_elapsed, SimDuration::from_ticks(1));
        assert_eq!(after_pause.remaining, SimDuration::from_ticks(1));

        let stale_progress = flow.step().unwrap().unwrap();
        assert_eq!(stale_progress.at, SimTime::from_ticks(7));
        assert!(stale_progress.error.is_none());
        let still_paused = flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .progress_at(SimTime::from_ticks(7))
            .unwrap();
        assert_eq!(still_paused.useful_elapsed, SimDuration::from_ticks(1));
        assert_eq!(still_paused.remaining, SimDuration::from_ticks(1));

        schedule_transit_control(
            &mut flow,
            carrier,
            TRANSIT_KIND,
            FlowDomainControl::Resume,
            SimTime::from_ticks(8),
            0,
        )
        .unwrap();
        assert!(flow.step().unwrap().unwrap().error.is_none());
        let arrival = flow.step().unwrap().unwrap();
        assert_eq!(arrival.at, SimTime::from_ticks(9));
        assert!(arrival.error.is_none());
        assert_eq!(
            flow.work_context::<TransitContext>(carrier)
                .unwrap()
                .phase(),
            TransitPhase::Arrived
        );
        let FlowBatchReceipt::Accepted(admissions) = &arrival.callback_batches[0] else {
            panic!("arrival must submit its actual timed service acquire");
        };
        assert_eq!(admissions.len(), 1);
        let request = admissions[0]
            .request
            .expect("actual request id in accepted receipt");
        assert_eq!(flow.work(service).unwrap().request, Some(request));
        let actual = flow.request(request).unwrap();
        assert_eq!(actual.owner, owner);
        assert_eq!(actual.work, Some(service));
        assert!(actual.timed);
        assert_ne!(actual.state, RequestState::Cancelled);
        assert!(flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .arrival_ticket()
            .is_some());
    }

    #[test]
    fn rejected_start_and_progress_recover_to_one_arrival_acquire() {
        const TRANSIT_KIND: EventKind = EventKind::custom(9413);
        PLANNER_CALLS.with(|calls| calls.set(0));
        let mut flow = FlowRuntime::new();
        flow.register_domain_plan_hook_with_receipt(
            "retry-once-transit-plan",
            TRANSIT_KIND,
            reject_start_and_first_progress_once,
            accept_transit_context,
        )
        .unwrap();
        let owner = flow.spawn_actor().unwrap();
        let carrier_actor = flow.spawn_actor().unwrap();
        let resource = flow.create_resource(1).unwrap();
        let service = flow
            .create_work(owner, SimDuration::from_ticks(4), "service", ())
            .unwrap();
        let start_at = SimTime::from_ticks(5);
        let carrier = flow
            .create_actor_domain_context(
                carrier_actor,
                "retry-once-transit-plan",
                TRANSIT_KIND,
                TransitContext::new(
                    &flow,
                    test_route(),
                    flow_acquire(resource, owner, service),
                    start_at,
                )
                .unwrap(),
            )
            .unwrap();
        let source_start =
            schedule_transit_start(&mut flow, carrier, TRANSIT_KIND, start_at, 4).unwrap();

        let rejected_start = flow.step().unwrap().unwrap();
        assert_eq!(rejected_start.event, source_start);
        assert_eq!(rejected_start.at, start_at);
        assert_eq!(rejected_start.error, Some(FlowError::InvalidState));
        assert!(matches!(
            rejected_start.callback_batches.as_slice(),
            [FlowBatchReceipt::Rejected(row)]
                if row.error == FlowError::InvalidState && row.failed_ticket.is_none()
        ));
        let ready = flow.work_context::<TransitContext>(carrier).unwrap();
        assert_eq!(ready.phase(), TransitPhase::Ready);
        assert_eq!(ready.expected_event, Some(source_start));
        assert_eq!(ready.expected_due, Some(start_at));

        let retry_start = TransitContext::schedule_transit_retry(
            &mut flow,
            carrier,
            TRANSIT_KIND,
            &rejected_start,
            4,
        )
        .unwrap();
        assert_ne!(retry_start, source_start);
        assert_eq!(
            flow.work_context::<TransitContext>(carrier)
                .unwrap()
                .expected_event,
            Some(retry_start)
        );
        let accepted_start = flow.step().unwrap().unwrap();
        assert_eq!(accepted_start.event, retry_start);
        assert_eq!(accepted_start.at, start_at);
        assert!(accepted_start.error.is_none());
        let FlowBatchReceipt::Accepted(start_rows) = &accepted_start.callback_batches[0] else {
            panic!("accepted retry start must schedule the first segment progress");
        };
        assert_eq!(start_rows.len(), 1);
        assert_eq!(start_rows[0].request, None);
        assert_eq!(
            flow.work_context::<TransitContext>(carrier)
                .unwrap()
                .phase(),
            TransitPhase::Moving
        );
        assert_eq!(
            TransitContext::schedule_transit_retry(
                &mut flow,
                carrier,
                TRANSIT_KIND,
                &rejected_start,
                4,
            ),
            Err(FlowError::InvalidWork)
        );

        let rejected_progress = flow.step().unwrap().unwrap();
        assert_eq!(rejected_progress.at, SimTime::from_ticks(6));
        assert_eq!(rejected_progress.error, Some(FlowError::InvalidState));
        assert!(matches!(
            rejected_progress.callback_batches.as_slice(),
            [FlowBatchReceipt::Rejected(row)]
                if row.error == FlowError::InvalidState && row.failed_ticket.is_none()
        ));
        let retained_progress = flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .progress_at(rejected_progress.at)
            .unwrap();
        assert_eq!(retained_progress.phase, TransitPhase::Moving);
        assert_eq!(retained_progress.segment_index, 1);
        assert_eq!(retained_progress.useful_elapsed, SimDuration::from_ticks(1));
        assert_eq!(retained_progress.remaining, SimDuration::from_ticks(1));
        let source_progress = rejected_progress.event;
        let retry_progress = TransitContext::schedule_transit_retry(
            &mut flow,
            carrier,
            TRANSIT_KIND,
            &rejected_progress,
            4,
        )
        .unwrap();
        assert_ne!(retry_progress, source_progress);
        assert_eq!(
            flow.work_context::<TransitContext>(carrier)
                .unwrap()
                .expected_event,
            Some(retry_progress)
        );
        let accepted_progress = flow.step().unwrap().unwrap();
        assert_eq!(accepted_progress.event, retry_progress);
        assert_eq!(accepted_progress.at, rejected_progress.at);
        assert!(accepted_progress.error.is_none());

        let arrival = flow.step().unwrap().unwrap();
        assert_eq!(arrival.at, SimTime::from_ticks(7));
        assert!(arrival.error.is_none());
        assert_eq!(
            flow.work_context::<TransitContext>(carrier)
                .unwrap()
                .phase(),
            TransitPhase::Arrived
        );
        let FlowBatchReceipt::Accepted(admissions) = &arrival.callback_batches[0] else {
            panic!("arrival after recovered progress must submit its actual timed acquire");
        };
        assert_eq!(admissions.len(), 1);
        let request = admissions[0]
            .request
            .expect("accepted actual acquire returns the request id");
        assert_eq!(flow.work(service).unwrap().request, Some(request));
        let actual = flow.request(request).unwrap();
        assert_eq!(actual.owner, owner);
        assert_eq!(actual.work, Some(service));
        assert!(actual.timed);
        assert_ne!(actual.state, RequestState::Cancelled);
        assert!(flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .arrival_ticket()
            .is_some());
    }

    #[test]
    fn same_tick_alien_domain_event_cannot_start_before_bound_start_id() {
        const TRANSIT_KIND: EventKind = EventKind::custom(9412);
        let mut flow = FlowRuntime::new();
        register_transit_context(&mut flow, "exact-start-plan", TRANSIT_KIND).unwrap();
        let owner = flow.spawn_actor().unwrap();
        let carrier_actor = flow.spawn_actor().unwrap();
        let resource = flow.create_resource(1).unwrap();
        let service = flow
            .create_work(owner, SimDuration::from_ticks(4), "service", ())
            .unwrap();
        let context = TransitContext::new(
            &flow,
            test_route(),
            flow_acquire(resource, owner, service),
            SimTime::from_ticks(5),
        )
        .unwrap();
        let carrier = flow
            .create_actor_domain_context(carrier_actor, "exact-start-plan", TRANSIT_KIND, context)
            .unwrap();
        let retained_start =
            schedule_transit_start(&mut flow, carrier, TRANSIT_KIND, SimTime::from_ticks(5), 0)
                .unwrap();
        let alien = flow
            .schedule_domain(carrier, TRANSIT_KIND, SimTime::from_ticks(5), -1)
            .unwrap();
        assert_ne!(retained_start, alien);

        let alien_dispatch = flow.step().unwrap().unwrap();
        assert_eq!(alien_dispatch.event, alien);
        assert!(alien_dispatch.error.is_none());
        assert!(matches!(
            alien_dispatch.callback_batches.as_slice(),
            [FlowBatchReceipt::Accepted(rows)] if rows.is_empty()
        ));
        let before_start = flow.work_context::<TransitContext>(carrier).unwrap();
        assert_eq!(before_start.phase(), TransitPhase::Ready);
        assert_eq!(
            before_start
                .progress_at(SimTime::from_ticks(5))
                .unwrap()
                .useful_elapsed,
            SimDuration::ZERO
        );

        let exact_dispatch = flow.step().unwrap().unwrap();
        assert_eq!(exact_dispatch.event, retained_start);
        assert!(exact_dispatch.error.is_none());
        assert_eq!(
            flow.work_context::<TransitContext>(carrier)
                .unwrap()
                .phase(),
            TransitPhase::Moving
        );
        assert_eq!(
            flow.work_context::<TransitContext>(carrier)
                .unwrap()
                .progress_at(SimTime::from_ticks(5))
                .unwrap()
                .useful_elapsed,
            SimDuration::ZERO
        );
    }

    #[test]
    fn rejected_arrival_batch_keeps_transit_context_and_existing_request_unchanged() {
        const TRANSIT_KIND: EventKind = EventKind::custom(9411);
        let mut flow = FlowRuntime::new();
        register_transit_context(&mut flow, "zero-transit-plan", TRANSIT_KIND).unwrap();
        let owner = flow.spawn_actor().unwrap();
        let carrier_actor = flow.spawn_actor().unwrap();
        let resource = flow.create_resource(1).unwrap();
        let service = flow
            .create_work(owner, SimDuration::from_ticks(2), "service", ())
            .unwrap();
        let context = TransitContext::new(
            &flow,
            zero_route(),
            flow_acquire(resource, owner, service),
            SimTime::from_ticks(5),
        )
        .unwrap();
        let existing = flow
            .submit_work(resource, owner, service, SimTime::from_ticks(5))
            .unwrap();
        let carrier = flow
            .create_actor_domain_context(carrier_actor, "zero-transit-plan", TRANSIT_KIND, context)
            .unwrap();
        schedule_transit_start(&mut flow, carrier, TRANSIT_KIND, SimTime::from_ticks(5), 0)
            .unwrap();

        let submit = flow.step().unwrap().unwrap();
        assert!(submit.error.is_none());
        let arrival = flow.step().unwrap().unwrap();
        assert_eq!(arrival.at, SimTime::from_ticks(5));
        assert!(arrival.error.is_none());
        assert!(
            matches!(arrival.callback_batches.as_slice(), [FlowBatchReceipt::Rejected(row)] if row.failed_ticket.is_some())
        );
        assert_eq!(flow.work(service).unwrap().request, Some(existing));
        assert_eq!(flow.request(existing).unwrap().work, Some(service));
        let retained = flow.work_context::<TransitContext>(carrier).unwrap();
        assert_eq!(retained.phase(), TransitPhase::Ready);
        assert_eq!(
            retained
                .progress_at(SimTime::from_ticks(5))
                .unwrap()
                .useful_elapsed,
            SimDuration::ZERO
        );
    }
}
