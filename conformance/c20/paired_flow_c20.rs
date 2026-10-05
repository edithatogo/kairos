use crate::flow_bridge::{
    AcquireIntent, BoundIntrinsicWork, PreparationIdentity, SubmittedIntrinsicWork, TransitRequest,
    WorkPreparationInput,
};
use crate::seed_map::{CalibrationSeedMap, CalibrationStreamKey, SeedPurpose};
use crate::work_duration::{IntrinsicDurationDistribution, IntrinsicWorkProvider};
use kairo_ecs_des::fidelity::{FidelityAdapter, FidelityMode, FidelityPolicy};
use kairo_ecs_des::{FlowContinuations, FlowRuntime, LifecycleTransition, RequestState, WorkState};
use kairo_ecs_types::{EntityId, SimDuration, SimTime};

const WORK_REGISTRATION: &str = "paired-work";
const SERVICE_TASK: &str = "triage:1";

#[derive(Clone, Debug, Eq, PartialEq)]
struct Template {
    marker: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Context {
    marker: u64,
}

fn make_context(template: &Template) -> Context {
    Context {
        marker: template.marker,
    }
}

fn provider() -> IntrinsicWorkProvider {
    IntrinsicWorkProvider::new(
        1,
        vec![(
            "triage".to_owned(),
            IntrinsicDurationDistribution::weighted_ticks(vec![(10, 3), (20, 2), (30, 5)]).unwrap(),
        )],
    )
    .unwrap()
}

struct Scenario {
    flow: FlowRuntime,
    bound: BoundIntrinsicWork<Template, Context>,
    owner: EntityId,
    carrier: EntityId,
    service_key: CalibrationStreamKey,
}

fn scenario(mode: FidelityMode) -> Scenario {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let carrier = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    flow.register_work_continuations(WORK_REGISTRATION, FlowContinuations::<Context>::default())
        .unwrap();

    let mut seeds = CalibrationSeedMap::new(1, "study-α", 1234).unwrap();
    let service_key = seeds
        .key_for("crn-v1", 7, "case-0001", SERVICE_TASK, SeedPurpose::Service)
        .unwrap();
    let expected = seeds
        .key_for("crn-v1", 7, "case-0001", SERVICE_TASK, SeedPurpose::Service)
        .unwrap();
    let service = seeds
        .stream_for("crn-v1", 7, "case-0001", SERVICE_TASK, SeedPurpose::Service)
        .unwrap();
    let policy = FidelityPolicy::new(1, Some(mode)).unwrap();
    let mut fidelity = FidelityAdapter::new(policy);
    let input = WorkPreparationInput::new(
        PreparationIdentity {
            owner,
            subsystem: "triage".to_owned(),
            registration: WORK_REGISTRATION.to_owned(),
            stratum: "triage".to_owned(),
        },
        service,
        expected,
        Template { marker: 0xC20 },
        make_context,
        AcquireIntent {
            resource,
            owner,
            at: SimTime::from_ticks(0),
            priority_level: 0,
            deadline: None,
            scheduler_priority: 0,
            can_preempt: false,
            preemptible: None,
        },
        TransitRequest::Zero,
    );
    let prepared = input
        .prepare(&flow, &mut fidelity, &provider())
        .unwrap_or_else(|_| panic!("provider preparation against actual Flow failed"));
    assert_eq!(prepared.sampled_duration(), SimDuration::from_ticks(30));
    assert_eq!(prepared.service_draw_position(), 1);
    let created = prepared
        .create(&mut flow)
        .unwrap_or_else(|_| panic!("actual restartable Flow work creation failed"));
    let work = created.work();
    let bound = created
        .bind(&flow)
        .unwrap_or_else(|_| panic!("actual Flow work binding failed"));
    assert_eq!(fidelity.decision(work).unwrap().mode, mode);
    assert_eq!(bound.sampled_duration(), SimDuration::from_ticks(30));
    assert_eq!(bound.service_draw_position(), 1);
    Scenario {
        flow,
        bound,
        owner,
        carrier,
        service_key,
    }
}

fn submit(
    flow: &mut FlowRuntime,
    bound: BoundIntrinsicWork<Template, Context>,
) -> SubmittedIntrinsicWork<Template, Context> {
    bound
        .submit(flow)
        .unwrap_or_else(|_| panic!("actual timed Flow acquire submission failed"))
}

fn assert_linked_request(
    flow: &FlowRuntime,
    submitted: &SubmittedIntrinsicWork<Template, Context>,
) {
    let work = submitted.work();
    let request = submitted.request();
    assert_eq!(flow.work(work).unwrap().request, Some(request));
    let actual = flow.request(request).unwrap();
    assert_eq!(actual.work, Some(work));
    assert!(actual.timed);
}

#[test]
fn macro_and_explicit_zero_micro_pair_actual_provider_work_without_transit_events_or_draws() {
    let mut macro_case = scenario(FidelityMode::Macro);
    let mut zero_micro_case = scenario(FidelityMode::Micro);
    assert!(macro_case.service_key == zero_micro_case.service_key);
    assert_eq!(
        macro_case.bound.sampled_duration(),
        SimDuration::from_ticks(30)
    );
    assert_eq!(
        zero_micro_case.bound.sampled_duration(),
        SimDuration::from_ticks(30)
    );
    assert_eq!(macro_case.bound.service_draw_position(), 1);
    assert_eq!(zero_micro_case.bound.service_draw_position(), 1);

    let macro_submitted = submit(&mut macro_case.flow, macro_case.bound);
    let zero_micro_submitted = submit(&mut zero_micro_case.flow, zero_micro_case.bound);
    let macro_work = macro_submitted.work();
    let zero_micro_work = zero_micro_submitted.work();
    let macro_request = macro_submitted.request();
    let zero_micro_request = zero_micro_submitted.request();
    let macro_run = macro_case.flow.run_for(128).unwrap();
    let zero_micro_run = zero_micro_case.flow.run_for(128).unwrap();

    assert!(macro_run
        .dispatches
        .iter()
        .all(|dispatch| dispatch.callback_batches.is_empty()));
    assert!(zero_micro_run
        .dispatches
        .iter()
        .all(|dispatch| dispatch.callback_batches.is_empty()));
    assert!(macro_case
        .flow
        .actor_domain_context(macro_case.owner)
        .is_err());
    assert!(zero_micro_case
        .flow
        .actor_domain_context(zero_micro_case.owner)
        .is_err());
    assert_eq!(macro_run.dispatches, zero_micro_run.dispatches);
    assert_eq!(
        macro_case.flow.work_progress(macro_work).unwrap(),
        zero_micro_case.flow.work_progress(zero_micro_work).unwrap()
    );
    assert_eq!(
        macro_case.flow.request(macro_request).unwrap(),
        zero_micro_case.flow.request(zero_micro_request).unwrap()
    );
    let progress = macro_case.flow.work_progress(macro_work).unwrap();
    assert_eq!(progress.state, WorkState::Completed);
    assert_eq!(progress.original_duration, SimDuration::from_ticks(30));
    assert_eq!(progress.completion_at, Some(SimTime::from_ticks(30)));
    assert_eq!(
        macro_case.flow.request(macro_request).unwrap().state,
        RequestState::Completed
    );
    assert_eq!(
        macro_case.flow.budget_snapshot().scheduler.scheduled_events,
        zero_micro_case
            .flow
            .budget_snapshot()
            .scheduler
            .scheduled_events
    );
    assert_eq!(
        macro_case
            .flow
            .budget_snapshot()
            .scheduler
            .dispatched_events,
        zero_micro_case
            .flow
            .budget_snapshot()
            .scheduler
            .dispatched_events
    );
    assert!(macro_case
        .flow
        .actor_domain_context(macro_case.carrier)
        .is_err());
    assert!(zero_micro_case
        .flow
        .actor_domain_context(zero_micro_case.carrier)
        .is_err());
    assert_eq!(
        macro_run
            .dispatches
            .iter()
            .flat_map(|dispatch| dispatch.records.iter())
            .filter(|record| record.transition == LifecycleTransition::Queued)
            .count(),
        1
    );
    assert_linked_request(&macro_case.flow, &macro_submitted);
    assert_linked_request(&zero_micro_case.flow, &zero_micro_submitted);
    assert_eq!(macro_submitted.service_draw_position(), 1);
    assert_eq!(zero_micro_submitted.service_draw_position(), 1);
}
