use kairo_ecs_des::{
    FlowError, FlowRuntime, LifecycleRecord, LifecycleTransition as L, PreemptionStrategy,
    RequestState, Resource, WorkState,
};
use kairo_ecs_types::{EntityId, SimDuration, SimTime};
fn t(n: u128) -> SimTime {
    SimTime::from_ticks(n)
}
fn duration(n: u128) -> SimDuration {
    SimDuration::from_ticks(n)
}
fn drain(f: &mut FlowRuntime) -> Vec<LifecycleRecord> {
    let mut rows = Vec::new();
    for _ in 0..64 {
        match f.step().unwrap() {
            Some(d) => {
                assert!(d.error.is_none(), "{:?}", d.error);
                rows.extend(d.records)
            }
            None => return rows,
        }
    }
    panic!("dispatch bound exceeded")
}
#[test]
fn defaults_and_explicit_policy_translate_to_request() {
    let mut f = FlowRuntime::new();
    let a = f.spawn_actor().unwrap();
    let r = f.create_resource(1).unwrap();
    let q = f.acquire(r).owner(a).submit().unwrap();
    let s = f.request(q).unwrap();
    assert_eq!(
        (
            s.owner,
            s.resource,
            s.submitted_at,
            s.priority_level,
            s.deadline,
            s.work
        ),
        (a, r, t(0), 0, None, None)
    );
    assert!(!s.timed);
    assert!(!s.can_preempt);
    assert_eq!(s.preemptible, None);
    assert_eq!(s.state, RequestState::Pending);
    let mut f = FlowRuntime::new();
    let a = f.spawn_actor().unwrap();
    let r = f.create_resource(1).unwrap();
    let w = f.create_work(a, duration(4), "policy", 42u32).unwrap();
    let q = f
        .acquire(r)
        .owner(a)
        .at(t(3))
        .timed_work(w)
        .priority(-7)
        .deadline(t(9))
        .can_preempt(true)
        .preemptible(PreemptionStrategy::Suspend)
        .scheduler_priority(-11)
        .submit()
        .unwrap();
    let s = f.request(q).unwrap();
    assert_eq!(
        (s.submitted_at, s.priority_level, s.deadline, s.work),
        (t(3), -7, Some(t(9)), Some(w))
    );
    assert!(s.timed && s.can_preempt);
    assert_eq!(s.preemptible, Some(PreemptionStrategy::Suspend));
    assert_eq!(f.work(w).unwrap().request, Some(q));
    let d = f.step().unwrap().unwrap();
    assert_eq!(d.at, t(3));
    assert_eq!(
        d.records.iter().map(|x| x.transition).collect::<Vec<_>>(),
        vec![L::Queued, L::Granted]
    );
    drain(&mut f);
    assert_eq!(f.request(q).unwrap().state, RequestState::Completed);
    assert_eq!(*f.work_context::<u32>(w).unwrap(), 42);
}
#[test]
fn failed_admission_preserves_association_and_full_public_scheduler_stats() {
    let mut f = FlowRuntime::new();
    let a = f.spawn_actor().unwrap();
    let b = f.spawn_actor().unwrap();
    let r = f.create_resource(1).unwrap();
    let w = f.create_work(a, duration(2), "negative", 17u32).unwrap();
    let before = f.budget_snapshot();
    assert_eq!(f.acquire(r).submit(), Err(FlowError::InvalidState));
    assert_eq!(f.budget_snapshot(), before);
    assert_eq!(
        f.acquire(r).owner(b).timed_work(w).submit(),
        Err(FlowError::InvalidWork)
    );
    assert_eq!(f.work(w).unwrap().request, None);
    assert_eq!(f.budget_snapshot(), before);
    assert_eq!(
        f.acquire(r)
            .owner(a)
            .for_work(w)
            .preemptible(PreemptionStrategy::Suspend)
            .submit(),
        Err(FlowError::InvalidWork)
    );
    assert_eq!(f.work(w).unwrap().request, None);
    assert_eq!(f.budget_snapshot(), before);
    let q = f.acquire(r).owner(a).for_work(w).at(t(3)).submit().unwrap();
    let before = f.budget_snapshot();
    assert_eq!(
        f.acquire(r).owner(a).for_work(w).submit(),
        Err(FlowError::InvalidWork)
    );
    assert_eq!(f.work(w).unwrap().request, Some(q));
    assert_eq!(f.budget_snapshot(), before);
    f.step().unwrap();
    let before = f.budget_snapshot();
    assert_eq!(
        f.acquire(r).owner(a).at(t(2)).submit(),
        Err(FlowError::PastCommand)
    );
    assert_eq!(f.budget_snapshot(), before);
    // Heap-head identity is not publicly exposed by FlowRuntime; no hidden-state claim.
}
#[test]
fn scheduler_admission_priority_is_distinct_from_resource_queue_priority() {
    let mut f = FlowRuntime::new();
    let a = f.spawn_actor().unwrap();
    let b = f.spawn_actor().unwrap();
    let r = f.create_resource(0).unwrap();
    let best = f
        .acquire(r)
        .owner(a)
        .priority(-9)
        .scheduler_priority(5)
        .submit()
        .unwrap();
    let early = f
        .acquire(r)
        .owner(b)
        .priority(9)
        .scheduler_priority(-5)
        .submit()
        .unwrap();
    assert_eq!(f.step().unwrap().unwrap().records[0].request, early);
    assert_eq!(f.step().unwrap().unwrap().records[0].request, best);
    assert!(
        f.request(early).unwrap().admission_sequence < f.request(best).unwrap().admission_sequence
    );
    f.set_capacity(r, 1).unwrap();
    let d = f.step().unwrap().unwrap();
    assert_eq!(
        d.records
            .iter()
            .map(|x| (x.request, x.transition))
            .collect::<Vec<_>>(),
        vec![(best, L::Granted)]
    );
    assert_eq!(f.request(early).unwrap().state, RequestState::Queued);
}
#[test]
fn deadline_at_completion_expires_without_replacement_grant_in_both_insertions() {
    for first in [false, true] {
        let mut f = FlowRuntime::new();
        let a = f.spawn_actor().unwrap();
        let b = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let w = f.create_work(a, duration(2), "deadline", ()).unwrap();
        let early = if first {
            Some(
                f.acquire(r)
                    .owner(b)
                    .at(t(1))
                    .deadline(t(2))
                    .submit()
                    .unwrap(),
            )
        } else {
            None
        };
        let holder = f.acquire(r).owner(a).timed_work(w).submit().unwrap();
        let waiter = early.unwrap_or_else(|| {
            f.acquire(r)
                .owner(b)
                .at(t(1))
                .deadline(t(2))
                .submit()
                .unwrap()
        });
        let rows = drain(&mut f);
        assert_eq!(f.request(holder).unwrap().state, RequestState::Completed);
        assert_eq!(f.request(waiter).unwrap().state, RequestState::TimedOut);
        assert!(!rows
            .iter()
            .any(|x| x.request == waiter && x.transition == L::Granted));
        assert_eq!(
            rows.iter()
                .filter(|x| x.request == waiter && x.transition == L::TimedOut)
                .count(),
            1
        );
    }
}
#[test]
fn fifo_migration_keeps_label_order_and_teaches_buffered_lease_release() {
    let ids = [
        EntityId::new(1, 0),
        EntityId::new(2, 0),
        EntityId::new(3, 0),
    ];
    let mut old = Resource::new("fifo", 1);
    assert!(old.request(ids[0]));
    assert!(!old.request(ids[1]));
    assert!(!old.request(ids[2]));
    assert_eq!(
        [ids[0], old.release().unwrap(), old.release().unwrap()],
        ids
    );
    assert_eq!(old.release(), None);
    let mut f = FlowRuntime::new();
    let actors = [
        f.spawn_actor().unwrap(),
        f.spawn_actor().unwrap(),
        f.spawn_actor().unwrap(),
    ];
    let r = f.create_resource(1).unwrap();
    let qs = actors.map(|a| f.acquire(r).owner(a).submit().unwrap());
    let mut rows = drain(&mut f);
    assert_eq!(f.request(qs[0]).unwrap().state, RequestState::Active);
    assert_eq!(f.request(qs[1]).unwrap().state, RequestState::Queued);
    for q in qs {
        let lease = f.request(q).unwrap().lease.unwrap();
        f.release(lease, t(0)).unwrap();
        assert_eq!(f.request(q).unwrap().state, RequestState::Active);
        rows.extend(drain(&mut f));
        assert_eq!(f.request(q).unwrap().state, RequestState::Released);
        let before = f.budget_snapshot();
        assert_eq!(f.release(lease, t(0)), Err(FlowError::InvalidLease));
        assert_eq!(f.budget_snapshot(), before);
    }
    assert_eq!(
        rows.iter()
            .filter(|x| x.transition == L::Granted)
            .map(|x| x.request)
            .collect::<Vec<_>>(),
        qs
    );
    assert!(f.resource(r).unwrap().active.is_empty());
}
fn interrupt(pause: bool) -> (Vec<LifecycleRecord>, String) {
    let mut f = FlowRuntime::new();
    let low = f.spawn_actor().unwrap();
    let urgent = f.spawn_actor().unwrap();
    let r = f.create_resource(1).unwrap();
    let lw = f
        .create_work(low, duration(10), "pause", String::from("low owned"))
        .unwrap();
    let uw = f
        .create_work(urgent, duration(2), "pause", String::from("urgent owned"))
        .unwrap();
    let l = f
        .acquire(r)
        .owner(low)
        .timed_work(lw)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    let u = f
        .acquire(r)
        .owner(urgent)
        .timed_work(uw)
        .can_preempt(true)
        .priority(-1)
        .at(t(3))
        .submit()
        .unwrap();
    let mut rows = Vec::new();
    if pause {
        for _ in 0..2 {
            let d = f.step().unwrap().unwrap();
            assert!(d.error.is_none());
            rows.extend(d.records);
        }
        assert_eq!(f.now(), t(3));
        assert_eq!(f.request(l).unwrap().state, RequestState::Suspended);
        assert_eq!(f.work_context::<String>(lw).unwrap(), "low owned");
        assert_eq!(f.work_progress(lw).unwrap().remaining, duration(7));
    }
    rows.extend(drain(&mut f));
    assert_eq!(f.request(l).unwrap().state, RequestState::Completed);
    assert_eq!(f.request(u).unwrap().state, RequestState::Completed);
    let p = f.work_progress(lw).unwrap();
    assert_eq!(p.state, WorkState::Completed);
    assert_eq!(p.cumulative_busy, duration(10));
    assert_eq!(p.useful_elapsed, duration(10));
    assert_eq!(p.remaining, duration(0));
    assert_eq!(f.work_progress(uw).unwrap().cumulative_busy, duration(2));
    assert_eq!(
        rows.iter()
            .filter(|x| x.request == l)
            .map(|x| (x.at.ticks(), x.transition))
            .collect::<Vec<_>>(),
        vec![
            (0, L::Queued),
            (0, L::Granted),
            (3, L::Preempted),
            (5, L::Resumed),
            (12, L::Completed)
        ]
    );
    (rows, f.work_context::<String>(lw).unwrap().clone())
}
#[test]
fn boundary_pause_retains_context_and_matches_uninterrupted_causal_records() {
    assert_eq!(interrupt(false), interrupt(true));
}
