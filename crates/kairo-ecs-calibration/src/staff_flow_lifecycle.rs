//! Private adapter from accepted staff selections to typed Flow requests and
//! owner-bound resource claims. Cleaning policy remains caller-owned.

use crate::staff_dispatch::{DispatchActivity, DispatchSelection};
use kairo_ecs_des::{
    FlowBatchReceipt, FlowCommandSink, FlowCommandTicket, FlowError, FlowOwnedCommand, FlowRuntime,
    LeaseId, RequestId, RequestState, ResourceId, WorkId,
};
use kairo_ecs_types::{EntityId, EventId, SimTime};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DispatchFlowMapping {
    pub(crate) dispatch_request_id: u64,
    pub(crate) dispatch_staff_id: u64,
    pub(crate) staff_actor: EntityId,
    pub(crate) resource: ResourceId,
    pub(crate) work: Option<WorkId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LifecycleError {
    InvalidMapping,
    Flow(FlowError),
}

impl From<FlowError> for LifecycleError {
    fn from(error: FlowError) -> Self {
        Self::Flow(error)
    }
}

/// Submit the selected Work or Walk using caller-provided typed Flow identities.
/// Wait is explicit and never creates a Flow request or resource claim.
pub(crate) fn submit_selection(
    flow: &mut FlowRuntime,
    selection: DispatchSelection,
    mapping: DispatchFlowMapping,
    at: SimTime,
) -> Result<Option<RequestId>, LifecycleError> {
    if selection.request_id != mapping.dispatch_request_id
        || selection.staff_id != mapping.dispatch_staff_id
    {
        return Err(LifecycleError::InvalidMapping);
    }
    match (selection.activity, mapping.work) {
        (DispatchActivity::Wait, None) => Ok(None),
        (DispatchActivity::Work | DispatchActivity::Walk, Some(work)) => flow
            .submit_work(mapping.resource, mapping.staff_actor, work, at)
            .map(Some)
            .map_err(LifecycleError::Flow),
        _ => Err(LifecycleError::InvalidMapping),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NamedBedClaim {
    pub(crate) name: String,
    pub(crate) owner: EntityId,
    pub(crate) resource: ResourceId,
    pub(crate) request: RequestId,
    pub(crate) lease: LeaseId,
    pub(crate) work: Option<WorkId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CleaningContext {
    pub(crate) claim: NamedBedClaim,
    pub(crate) completion_work: Option<WorkId>,
    pub(crate) callback_event: Option<EventId>,
    pub(crate) release_ticket: Option<FlowCommandTicket>,
    pub(crate) release_event: Option<EventId>,
}

/// Caller registers this planner for its cleaning-completion EventKind. It
/// releases only the exact lease after the expected callback identity arrives.
pub(crate) fn cleaning_completion(
    context: &CleaningContext,
    snapshot: &kairo_ecs_des::FlowCallbackSnapshot,
    world: kairo_ecs_des::FlowWorldView<'_>,
    sink: &mut FlowCommandSink,
) -> Result<CleaningContext, FlowError> {
    if context.callback_event != Some(snapshot.origin)
        || !matches!(
            snapshot.cause,
            kairo_ecs_des::FlowCallbackCause::Domain { .. }
        )
        || !world.is_alive(context.claim.owner)
        || context.claim.name.is_empty()
        || context
            .completion_work
            .is_some_and(|work| work != snapshot.work)
    {
        return Err(FlowError::InvalidWork);
    }

    let mut candidate = context.clone();
    candidate.completion_work = Some(snapshot.work);
    candidate.release_ticket = Some(sink.emit(FlowOwnedCommand::Release {
        lease: context.claim.lease,
        at: world.now(),
    })?);
    Ok(candidate)
}

pub(crate) fn record_accepted_release(context: &mut CleaningContext, receipt: &FlowBatchReceipt) {
    let FlowBatchReceipt::Accepted(admissions) = receipt else {
        return;
    };
    context.release_event = context.release_ticket.and_then(|ticket| {
        admissions
            .iter()
            .find(|admission| admission.ticket == ticket)
            .map(|admission| admission.event)
    });
}

pub(crate) fn active_claim(
    flow: &FlowRuntime,
    name: impl Into<String>,
    owner: EntityId,
    resource: ResourceId,
    request: RequestId,
    work: Option<WorkId>,
) -> Result<NamedBedClaim, LifecycleError> {
    let state = flow.request(request)?;
    if state.state != RequestState::Active
        || state.owner != owner
        || state.resource != resource
        || state.work != work
        || state.lease.is_none()
    {
        return Err(LifecycleError::InvalidMapping);
    }
    Ok(NamedBedClaim {
        name: name.into(),
        owner,
        resource,
        request,
        lease: state.lease.expect("active request has a lease"),
        work: state.work,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::staff_dispatch::{select_dispatch, DispatchRequest, StaffMember};
    use kairo_ecs_types::{EventKind, SimDuration};
    use std::collections::BTreeSet;

    const CLEANING_KIND: EventKind = EventKind::custom(0xC2_30);
    const REGISTRATION: &str = "staff-flow-lifecycle.cleaning";

    fn ticks(value: u128) -> SimTime {
        SimTime::from_ticks(value)
    }

    fn dispatch_request(id: u64, activity: DispatchActivity) -> DispatchRequest {
        DispatchRequest {
            id,
            urgency: 1,
            fifo_sequence: id,
            assigned_zone: "ward".to_owned(),
            required_skills: BTreeSet::new(),
            activity,
        }
    }

    fn staff(id: u64) -> StaffMember {
        StaffMember {
            id,
            zone: "ward".to_owned(),
            skills: BTreeSet::new(),
            available: true,
        }
    }

    fn bind_event(context: &mut CleaningContext, event: EventId) {
        context.callback_event = Some(event);
    }

    #[test]
    fn selected_work_walk_wait_and_cleaning_receipt_preserve_claim_lineage() {
        let mut flow = FlowRuntime::new();
        let owner = flow.spawn_actor().unwrap();
        let walk_owner = flow.spawn_actor().unwrap();
        let queued_owner = flow.spawn_actor().unwrap();
        let staff_resource = flow.create_resource(1).unwrap();
        let walk_staff_resource = flow.create_resource(1).unwrap();
        let bed = flow.create_resource(1).unwrap();
        let walk_resource = flow.create_resource(1).unwrap();
        let wait_resource = flow.create_resource(1).unwrap();

        flow.register_work_handlers::<()>("staff-flow-lifecycle.activity", Default::default())
            .unwrap();
        flow.register_domain_plan_hook_with_receipt::<CleaningContext>(
            REGISTRATION,
            CLEANING_KIND,
            cleaning_completion,
            record_accepted_release,
        )
        .unwrap();
        let work = flow
            .create_work(
                owner,
                SimDuration::from_ticks(5),
                "staff-flow-lifecycle.activity",
                (),
            )
            .unwrap();
        let walk = flow
            .create_work(
                walk_owner,
                SimDuration::from_ticks(5),
                "staff-flow-lifecycle.activity",
                (),
            )
            .unwrap();

        // Staff allocation is a distinct first stage; bed admission follows it.
        let staff_request = flow.submit(staff_resource, owner, ticks(0)).unwrap();
        let staff_admission = flow.step().unwrap().unwrap();
        assert_eq!(staff_admission.records[0].request, staff_request);
        assert_eq!(
            flow.request(staff_request).unwrap().state,
            RequestState::Active
        );
        let staff_claim = flow.resource(staff_resource).unwrap();
        assert_eq!(staff_claim.total, 1);
        assert_eq!(staff_claim.available + staff_claim.active.len() as u32, 1);
        assert_eq!(staff_claim.allocations.len(), 1);
        assert_eq!(staff_claim.allocations[0].request, staff_request);
        assert_eq!(staff_claim.allocations[0].owner, owner);

        let selected_work = select_dispatch(
            &[dispatch_request(101, DispatchActivity::Work)],
            &[staff(201)],
        )
        .unwrap()
        .unwrap();
        assert_eq!(selected_work.activity, DispatchActivity::Work);
        let bed_request = submit_selection(
            &mut flow,
            selected_work,
            DispatchFlowMapping {
                dispatch_request_id: 101,
                dispatch_staff_id: 201,
                staff_actor: owner,
                resource: bed,
                work: Some(work),
            },
            ticks(0),
        )
        .unwrap()
        .unwrap();
        let bed_admission = flow.step().unwrap().unwrap();
        assert_eq!(bed_admission.records[0].request, bed_request);
        let initial = flow.request(bed_request).unwrap();
        assert_eq!(initial.owner, owner);
        assert_eq!(initial.resource, bed);
        assert_eq!(initial.work, Some(work));
        assert_eq!(initial.state, RequestState::Active);
        let claim = active_claim(&flow, "Bed 4", owner, bed, bed_request, Some(work)).unwrap();

        let queued_request = flow.submit(bed, queued_owner, ticks(0)).unwrap();
        flow.step().unwrap().unwrap();
        let held = flow.resource(bed).unwrap();
        assert_eq!(held.total, 1);
        assert_eq!(held.available + held.active.len() as u32, held.total);
        assert_eq!(held.active, vec![claim.lease]);
        assert_eq!(held.queued, vec![queued_request]);

        // The walk mapping remains distinct and keeps caller-provided typed IDs.
        let walk_staff_request = flow
            .submit(walk_staff_resource, walk_owner, ticks(0))
            .unwrap();
        flow.step().unwrap().unwrap();
        assert_eq!(
            flow.request(walk_staff_request).unwrap().state,
            RequestState::Active
        );
        let walk_staff_claim = flow.resource(walk_staff_resource).unwrap();
        assert_eq!(walk_staff_claim.total, 1);
        assert_eq!(
            walk_staff_claim.available + walk_staff_claim.active.len() as u32,
            1
        );
        assert_eq!(walk_staff_claim.allocations.len(), 1);
        assert_eq!(walk_staff_claim.allocations[0].request, walk_staff_request);
        assert_eq!(walk_staff_claim.allocations[0].owner, walk_owner);
        let selected_walk = select_dispatch(
            &[dispatch_request(102, DispatchActivity::Walk)],
            &[staff(202)],
        )
        .unwrap()
        .unwrap();
        assert_eq!(selected_walk.activity, DispatchActivity::Walk);
        let walk_request = submit_selection(
            &mut flow,
            selected_walk,
            DispatchFlowMapping {
                dispatch_request_id: 102,
                dispatch_staff_id: 202,
                staff_actor: walk_owner,
                resource: walk_resource,
                work: Some(walk),
            },
            ticks(0),
        )
        .unwrap()
        .unwrap();
        flow.step().unwrap().unwrap();
        let walk_lineage = flow.request(walk_request).unwrap();
        assert_eq!(walk_lineage.owner, walk_owner);
        assert_eq!(walk_lineage.resource, walk_resource);
        assert_eq!(walk_lineage.work, Some(walk));
        assert_eq!(walk_lineage.state, RequestState::Active);

        // Wait is skipped by selection and the explicit adapter path emits none.
        assert!(select_dispatch(
            &[dispatch_request(103, DispatchActivity::Wait)],
            &[staff(203)],
        )
        .unwrap()
        .is_none());
        assert_eq!(
            submit_selection(
                &mut flow,
                DispatchSelection {
                    request_id: 103,
                    staff_id: 203,
                    activity: DispatchActivity::Wait,
                },
                DispatchFlowMapping {
                    dispatch_request_id: 103,
                    dispatch_staff_id: 203,
                    staff_actor: owner,
                    resource: wait_resource,
                    work: None,
                },
                ticks(0),
            )
            .unwrap(),
            None
        );
        let wait_state = flow.resource(wait_resource).unwrap();
        assert!(wait_state.active.is_empty());
        assert!(wait_state.queued.is_empty());
        assert_eq!(wait_state.available, wait_state.total);

        // The cleaning callback work is explicitly linked to the same typed claim.
        let cleaning_work = flow
            .create_work(
                owner,
                SimDuration::from_ticks(1),
                REGISTRATION,
                CleaningContext {
                    claim: claim.clone(),
                    completion_work: None,
                    callback_event: None,
                    release_ticket: None,
                    release_event: None,
                },
            )
            .unwrap();
        let callback_event = flow
            .schedule_domain_and_bind::<CleaningContext>(
                cleaning_work,
                CLEANING_KIND,
                ticks(2),
                0,
                bind_event,
            )
            .unwrap();
        // A same-work callback at an earlier EventId is not the bound completion.
        let wrong_callback_event = flow
            .schedule_domain(cleaning_work, CLEANING_KIND, ticks(1), 0)
            .unwrap();
        assert_ne!(wrong_callback_event, callback_event);

        let wrong_callback = flow.step().unwrap().unwrap();
        assert_eq!(wrong_callback.event, wrong_callback_event);
        assert!(matches!(
            wrong_callback.callback_batches.as_slice(),
            [FlowBatchReceipt::Rejected(_)]
        ));
        let after_rejection = flow.resource(bed).unwrap();
        assert_eq!(after_rejection.total, 1);
        assert_eq!(
            after_rejection.available + after_rejection.active.len() as u32,
            after_rejection.total
        );
        assert_eq!(after_rejection.active, vec![claim.lease]);
        assert_eq!(after_rejection.queued, vec![queued_request]);
        assert_eq!(
            flow.request(bed_request).unwrap().state,
            RequestState::Active
        );

        let callback = flow.step().unwrap().unwrap();
        assert_eq!(callback.event, callback_event);
        assert!(matches!(
            callback.callback_batches.as_slice(),
            [FlowBatchReceipt::Accepted(_)]
        ));
        let context = flow.work_context::<CleaningContext>(cleaning_work).unwrap();
        assert_eq!(context.callback_event, Some(callback_event));
        assert_eq!(context.claim.name, claim.name);
        assert_eq!(context.claim.owner, claim.owner);
        assert_eq!(context.claim.resource, claim.resource);
        assert_eq!(context.claim.request, claim.request);
        assert_eq!(context.claim.lease, claim.lease);
        assert_eq!(context.claim.work, Some(work));
        assert_eq!(context.completion_work, Some(cleaning_work));
        let release_event = context
            .release_event
            .expect("accepted release has exact event");
        let receipt = match &callback.callback_batches[0] {
            FlowBatchReceipt::Accepted(admissions) => admissions,
            FlowBatchReceipt::Rejected(_) => unreachable!(),
        };
        assert_eq!(receipt.len(), 1);
        assert_eq!(receipt[0].ticket, context.release_ticket.unwrap());
        assert_eq!(receipt[0].event, release_event);

        // Acceptance schedules the command; only stepping its exact event releases.
        let before_release = flow.resource(bed).unwrap();
        assert_eq!(before_release.total, 1);
        assert_eq!(
            before_release.available + before_release.active.len() as u32,
            before_release.total
        );
        assert_eq!(before_release.active, vec![claim.lease]);
        assert_eq!(before_release.queued, vec![queued_request]);
        assert_eq!(
            flow.request(bed_request).unwrap().state,
            RequestState::Active
        );
        let released = flow.step().unwrap().unwrap();
        assert_eq!(released.event, release_event);
        assert_eq!(
            flow.request(bed_request).unwrap().state,
            RequestState::Released
        );
        assert_eq!(
            flow.request(queued_request).unwrap().state,
            RequestState::Active
        );
        let after_release = flow.resource(bed).unwrap();
        assert_eq!(after_release.total, 1);
        assert_eq!(
            after_release.available + after_release.active.len() as u32,
            1
        );
        assert_eq!(
            after_release.active,
            flow.request(queued_request)
                .unwrap()
                .lease
                .into_iter()
                .collect::<Vec<_>>()
        );
        assert!(after_release.queued.is_empty());
    }
}
