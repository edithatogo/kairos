use super::*;

#[test]
fn despawn_plan_stages_only_affected_records_with_many_unrelated_rows() {
    let mut runtime = FlowRuntime::new();
    let owner = runtime.spawn_actor().unwrap();
    let resource = runtime.create_resource(1).unwrap();
    let work = runtime
        .create_work(owner, SimDuration::from_ticks(10), "q52.target", ())
        .unwrap();
    let request = runtime
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .submit()
        .unwrap();
    runtime.step().unwrap().unwrap();

    let unrelated_owner = runtime.spawn_actor().unwrap();
    let unrelated_resource = runtime.create_resource(0).unwrap();
    for _ in 0..1_000 {
        let unrelated_work = runtime
            .create_work(
                unrelated_owner,
                SimDuration::from_ticks(1),
                "q52.unrelated",
                (),
            )
            .unwrap();
        runtime
            .acquire(unrelated_resource)
            .owner(unrelated_owner)
            .timed_work(unrelated_work)
            .submit()
            .unwrap();
    }
    assert_eq!(runtime.requests.len(), 1_001);
    assert_eq!(runtime.works.len(), 1_001);

    let before_request = runtime.request(request).unwrap();
    let before_progress = runtime.work_progress(work).unwrap();
    let before_resource = runtime.resource(resource).unwrap();
    let before_world = runtime.world.snapshot();
    let before_scheduler = runtime.scheduler.stats();
    let before_head = runtime.scheduler.peek_next();
    let before_budget = (
        runtime.budget_tick,
        runtime.budget_consumed,
        runtime.budget_halt,
    );
    let before_counters = (runtime.created, runtime.scheduled, runtime.destroyed);
    let mut outcome = FlowDispatch {
        event: EventId::new(0, 0),
        at: SimTime::from_ticks(1),
        records: Vec::new(),
        error: None,
        callback_batches: Vec::new(),
    };

    let plan = runtime
        .plan(Command::Despawn(owner), &mut outcome)
        .unwrap()
        .expect("valid despawn plan");

    assert_eq!(plan.requests.len(), 1);
    assert_eq!(plan.requests.keys().copied().collect::<Vec<_>>(), [request]);
    assert_eq!(plan.progress.len(), 1);
    assert_eq!(plan.progress.keys().copied().collect::<Vec<_>>(), [work]);
    assert_eq!(plan.requests[&request].state, RequestState::Cancelled);
    assert_eq!(plan.progress[&work].state, WorkState::Cancelled);
    assert_eq!(runtime.request(request).unwrap(), before_request);
    assert_eq!(runtime.work_progress(work).unwrap(), before_progress);
    assert_eq!(runtime.resource(resource).unwrap(), before_resource);
    assert_eq!(runtime.world.snapshot(), before_world);
    assert_eq!(runtime.scheduler.stats(), before_scheduler);
    assert_eq!(runtime.scheduler.peek_next(), before_head);
    assert_eq!(
        (
            runtime.budget_tick,
            runtime.budget_consumed,
            runtime.budget_halt
        ),
        before_budget
    );
    assert_eq!(
        (runtime.created, runtime.scheduled, runtime.destroyed),
        before_counters
    );
}
