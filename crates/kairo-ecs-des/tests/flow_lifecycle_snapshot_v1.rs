use kairo_ecs_des::{
    FlowRuntime, LifecycleRecord, LifecycleTransition as T, PreemptionStrategy, RequestState,
};
use kairo_ecs_types::{SimDuration, SimTime};

fn t(n: u128) -> SimTime {
    SimTime::from_ticks(n)
}

fn d(n: u128) -> SimDuration {
    SimDuration::from_ticks(n)
}

fn restart_context(template: &u32) -> u32 {
    *template
}

fn drain(flow: &mut FlowRuntime) -> Vec<LifecycleRecord> {
    let mut records = Vec::new();
    for _ in 0..64 {
        match flow.step().unwrap() {
            Some(dispatch) => {
                assert!(dispatch.error.is_none(), "{:?}", dispatch.error);
                records.extend(dispatch.records);
            }
            None => return records,
        }
    }
    panic!("dispatch bound exceeded");
}

#[test]
fn public_handle_getters_and_zero_duration_rows_capture_each_mutation() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let work = flow.create_work(owner, d(0), "snapshot.zero", ()).unwrap();
    let request = flow
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .submit()
        .unwrap();
    let rows = drain(&mut flow);
    let _resource_entity = resource.entity_id();
    let _request_entity = request.entity_id();
    let _work_entity = work.entity_id();
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows.iter().map(|r| r.transition).collect::<Vec<_>>(),
        vec![T::Queued, T::Granted, T::Completed]
    );
    assert!(rows
        .iter()
        .all(|r| r.causal_event_id == rows[0].causal_event_id));
    assert_eq!(
        rows.iter()
            .map(|r| r.transition_ordinal)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );

    let queued = &rows[0].snapshot;
    assert_eq!(
        (queued.capacity, queued.queue_len, queued.active_count),
        (1, 1, 0)
    );
    assert_eq!(queued.owner, owner);
    assert_eq!(queued.work, Some(work));
    assert_eq!(queued.priority_level, 0);
    assert_eq!(queued.strategy, None);
    assert_eq!(queued.preemptor_request, None);
    assert_eq!(queued.causal_lease, None);
    assert_eq!(queued.progress.as_ref().unwrap().remaining, d(0));

    let granted = &rows[1].snapshot;
    assert_eq!(
        (granted.capacity, granted.queue_len, granted.active_count),
        (1, 0, 1)
    );
    assert_eq!(granted.causal_lease, rows[1].lease);
    assert_eq!(granted.causal_lease.unwrap().request_id(), request);
    assert_eq!(granted.causal_lease.unwrap().revision(), 0);
    assert_eq!(
        granted.progress.as_ref().unwrap().state,
        kairo_ecs_des::WorkState::Active
    );

    let completed = &rows[2].snapshot;
    assert_eq!(
        (
            completed.capacity,
            completed.queue_len,
            completed.active_count
        ),
        (1, 0, 0)
    );
    assert_eq!(completed.causal_lease, rows[1].lease);
    assert_eq!(
        completed.progress.as_ref().unwrap().state,
        kairo_ecs_des::WorkState::Completed
    );
    assert_eq!(completed.progress.as_ref().unwrap().remaining, d(0));
}

#[test]
fn suspend_snapshot_keeps_victim_lease_progress_and_preemptor_before_requeue() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let victim_work = flow
        .create_work(owner, d(10), "snapshot.suspend", ())
        .unwrap();
    let victim = flow
        .acquire(resource)
        .owner(owner)
        .priority(9)
        .timed_work(victim_work)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    let incoming_work = flow
        .create_work(owner, d(2), "snapshot.incoming", ())
        .unwrap();
    let incoming = flow
        .acquire(resource)
        .owner(owner)
        .at(t(3))
        .priority(1)
        .timed_work(incoming_work)
        .can_preempt(true)
        .submit()
        .unwrap();

    let initial = flow.step().unwrap().unwrap();
    let admission = flow.step().unwrap().unwrap();
    let row = admission
        .records
        .iter()
        .find(|row| row.request == victim && row.transition == T::Preempted)
        .unwrap();
    assert_eq!(row.at, t(3));
    assert_eq!(row.state, RequestState::Suspended);
    assert_eq!(row.lease, None);
    assert_eq!(row.snapshot.causal_lease.unwrap().request_id(), victim);
    assert_eq!(row.snapshot.causal_lease.unwrap().revision(), 0);
    assert_eq!(row.snapshot.preemptor_request, Some(incoming));
    assert_eq!(row.snapshot.strategy, Some(PreemptionStrategy::Suspend));
    assert_eq!(
        (
            row.snapshot.capacity,
            row.snapshot.queue_len,
            row.snapshot.active_count
        ),
        (1, 1, 0)
    );
    let progress = row.snapshot.progress.as_ref().unwrap();
    assert_eq!(progress.useful_elapsed, d(3));
    assert_eq!(progress.remaining, d(7));
    assert_eq!(progress.cumulative_busy, d(3));
    assert_eq!(progress.state, kairo_ecs_des::WorkState::Suspended);

    // The saved row remains the tick-3 state after the victim is requeued and resumed.
    let _later = drain(&mut flow);
    assert_eq!(row.snapshot.queue_len, 1);
    assert_eq!(row.snapshot.active_count, 0);
    assert_eq!(flow.request(victim).unwrap().state, RequestState::Completed);
    assert_eq!(
        flow.request(incoming).unwrap().state,
        RequestState::Completed
    );
    assert_eq!(initial.at, t(0));
}

#[test]
fn abort_emits_preempted_removal_then_aborted_state_with_same_causal_lease() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let work = flow
        .create_work(owner, d(10), "snapshot.abort", ())
        .unwrap();
    let victim = flow
        .acquire(resource)
        .owner(owner)
        .priority(9)
        .timed_work(work)
        .preemptible(PreemptionStrategy::Abort)
        .submit()
        .unwrap();
    let incoming_work = flow
        .create_work(owner, d(2), "snapshot.abort.incoming", ())
        .unwrap();
    let incoming = flow
        .acquire(resource)
        .owner(owner)
        .at(t(3))
        .priority(1)
        .timed_work(incoming_work)
        .can_preempt(true)
        .submit()
        .unwrap();
    flow.step().unwrap();
    let admission = flow.step().unwrap().unwrap();
    let victim_rows: Vec<_> = admission
        .records
        .iter()
        .filter(|row| row.request == victim)
        .collect();
    assert_eq!(
        victim_rows.iter().map(|r| r.transition).collect::<Vec<_>>(),
        vec![T::Preempted, T::Aborted]
    );
    let preempted = victim_rows[0];
    let aborted = victim_rows[1];
    assert_eq!(
        preempted.snapshot.causal_lease,
        aborted.snapshot.causal_lease
    );
    assert_eq!(preempted.snapshot.preemptor_request, Some(incoming));
    assert_eq!(preempted.snapshot.strategy, Some(PreemptionStrategy::Abort));
    assert_eq!(
        preempted.snapshot.progress.as_ref().unwrap().useful_elapsed,
        d(3)
    );
    assert_eq!(aborted.state, RequestState::Aborted);
    assert_eq!(
        aborted.snapshot.progress.as_ref().unwrap().state,
        kairo_ecs_des::WorkState::Aborted
    );
    assert_eq!(
        (
            preempted.snapshot.queue_len,
            preempted.snapshot.active_count
        ),
        (1, 0)
    );
    assert_eq!(
        (aborted.snapshot.queue_len, aborted.snapshot.active_count),
        (1, 0)
    );
}

#[test]
fn restart_snapshot_preserves_pre_reset_progress_and_records_new_attempt() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let work = flow
        .create_restartable_work(owner, d(10), "snapshot.restart", 7_u32, restart_context)
        .unwrap();
    let victim = flow
        .acquire(resource)
        .owner(owner)
        .priority(9)
        .timed_work(work)
        .preemptible(PreemptionStrategy::Restart)
        .submit()
        .unwrap();
    let urgent_work = flow
        .create_work(owner, d(2), "snapshot.restart.urgent", ())
        .unwrap();
    flow.acquire(resource)
        .owner(owner)
        .at(t(3))
        .priority(1)
        .timed_work(urgent_work)
        .can_preempt(true)
        .submit()
        .unwrap();
    flow.step().unwrap();
    let interrupted = flow.step().unwrap().unwrap();
    let preempted = interrupted
        .records
        .iter()
        .find(|row| row.request == victim && row.transition == T::Preempted)
        .unwrap();
    assert_eq!(
        preempted.snapshot.strategy,
        Some(PreemptionStrategy::Restart)
    );
    let old_progress = preempted.snapshot.progress.as_ref().unwrap();
    assert_eq!(
        (old_progress.useful_elapsed, old_progress.remaining),
        (d(3), d(7))
    );
    assert_eq!(old_progress.cumulative_busy, d(3));
    assert_eq!(old_progress.attempt_revision, 0);

    let restarted = drain(&mut flow)
        .into_iter()
        .find(|row| row.request == victim && row.transition == T::Restarted)
        .unwrap();
    let new_progress = restarted.snapshot.progress.as_ref().unwrap();
    assert_eq!(
        (new_progress.useful_elapsed, new_progress.remaining),
        (d(0), d(10))
    );
    assert_eq!(new_progress.cumulative_busy, d(3));
    assert_eq!(new_progress.attempt_revision, 1);
    assert_eq!(new_progress.state, kairo_ecs_des::WorkState::Active);
}

#[test]
fn release_and_cancel_snapshot_checkpointed_progress_and_removed_lease() {
    for cancel in [false, true] {
        let mut flow = FlowRuntime::new();
        let owner = flow.spawn_actor().unwrap();
        let resource = flow.create_resource(1).unwrap();
        let work = flow
            .create_work(owner, d(5), "snapshot.terminal", ())
            .unwrap();
        let request = flow
            .acquire(resource)
            .owner(owner)
            .timed_work(work)
            .submit()
            .unwrap();
        flow.step().unwrap();
        let lease = flow.request(request).unwrap().lease.unwrap();
        if cancel {
            flow.cancel(request, t(2)).unwrap();
        } else {
            flow.release(lease, t(2)).unwrap();
        }
        let terminal = flow.step().unwrap().unwrap();
        let row = terminal
            .records
            .iter()
            .find(|row| row.request == request)
            .unwrap();
        assert_eq!(
            row.transition,
            if cancel { T::Cancelled } else { T::Released }
        );
        assert_eq!(row.lease, None);
        assert_eq!(row.snapshot.causal_lease, Some(lease));
        let progress = row.snapshot.progress.as_ref().unwrap();
        assert_eq!((progress.useful_elapsed, progress.remaining), (d(2), d(3)));
        assert_eq!(progress.cumulative_busy, d(2));
        assert_eq!(
            progress.state,
            if cancel {
                kairo_ecs_des::WorkState::Cancelled
            } else {
                kairo_ecs_des::WorkState::Released
            }
        );

        let later = drain(&mut flow);
        assert!(!later
            .iter()
            .any(|r| r.request == request && r.transition == T::Completed));
        assert_eq!(row.snapshot.progress.as_ref().unwrap().remaining, d(3));
    }
}

#[test]
fn rejected_same_tick_plan_emits_no_partial_rows_or_state_changes() {
    use kairo_ecs_des::FlowConfig;
    use std::num::NonZeroU64;

    let mut flow = FlowRuntime::with_config(FlowConfig {
        max_same_tick_flow_transitions: NonZeroU64::new(1).unwrap(),
    });
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let request = flow.acquire(resource).owner(owner).submit().unwrap();
    assert!(matches!(
        flow.step(),
        Err(kairo_ecs_des::FlowError::SameTickBudgetExceeded { .. })
    ));
    assert_eq!(flow.request(request).unwrap().state, RequestState::Pending);
    let snapshot = flow.resource(resource).unwrap();
    assert!(snapshot.active.is_empty());
    assert!(snapshot.queued.is_empty());
}

fn lifecycle_run(paused: bool) -> Vec<LifecycleRecord> {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let work = flow.create_work(owner, d(3), "snapshot.pause", ()).unwrap();
    flow.acquire(resource)
        .owner(owner)
        .timed_work(work)
        .submit()
        .unwrap();
    let mut records = if paused {
        flow.run_for(1)
            .unwrap()
            .dispatches
            .into_iter()
            .flat_map(|d| d.records)
            .collect()
    } else {
        Vec::new()
    };
    records.extend(drain(&mut flow));
    records
}

#[test]
fn same_runtime_pause_preserves_canonical_lifecycle_snapshots() {
    assert_eq!(lifecycle_run(false), lifecycle_run(true));
}

#[test]
fn manual_and_actor_cleanup_rows_keep_null_work_or_pre_despawn_work_identity() {
    let mut flow = FlowRuntime::new();
    let manual_owner = flow.spawn_actor().unwrap();
    let cleanup_owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(2).unwrap();
    let manual = flow.acquire(resource).owner(manual_owner).submit().unwrap();
    let work = flow
        .create_work(cleanup_owner, d(5), "snapshot.cleanup", ())
        .unwrap();
    let task = flow
        .acquire(resource)
        .owner(cleanup_owner)
        .timed_work(work)
        .submit()
        .unwrap();
    let manual_grant = flow.step().unwrap().unwrap();
    let manual_row = manual_grant
        .records
        .iter()
        .find(|row| row.request == manual)
        .unwrap();
    assert_eq!(manual_row.snapshot.work, None);
    assert_eq!(manual_row.snapshot.progress, None);
    let task_grant = flow.step().unwrap().unwrap();
    let task_lease = task_grant
        .records
        .iter()
        .find(|row| row.request == task && row.transition == T::Granted)
        .unwrap()
        .lease
        .unwrap();
    flow.despawn_actor(cleanup_owner).unwrap();
    let cleanup = flow.step().unwrap().unwrap();
    let row = cleanup
        .records
        .iter()
        .find(|row| row.request == task)
        .unwrap();
    assert_eq!(row.transition, T::Cancelled);
    assert_eq!(row.snapshot.work, Some(work));
    assert_eq!(
        row.snapshot.progress.as_ref().unwrap().state,
        kairo_ecs_des::WorkState::Cancelled
    );
    assert_eq!(row.snapshot.causal_lease, Some(task_lease));
    assert_eq!(row.snapshot.causal_lease.unwrap().request_id(), task);
    assert_eq!(flow.work(work), Err(kairo_ecs_des::FlowError::InvalidWork));
}
