use kairo_ecs_des::{FlowError, FlowRuntime, RequestState};
use kairo_ecs_types::SimTime;
fn t(n: u128) -> SimTime {
    SimTime::from_ticks(n)
}
#[test]
fn priority_fifo_and_rekey_preserve_admission_sequence() {
    for rekey in [false, true] {
        let mut f = FlowRuntime::new();
        let o = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let h = f.submit(r, o, t(0)).unwrap();
        f.step().unwrap();
        let a = f.acquire(r).owner(o).priority(5).submit().unwrap();
        let b = f.acquire(r).owner(o).priority(1).submit().unwrap();
        let c = f.acquire(r).owner(o).priority(1).submit().unwrap();
        f.run_for(3).unwrap();
        let seq = f.request(a).unwrap().admission_sequence;
        if rekey {
            f.reprioritize(a, 1, t(0)).unwrap();
            f.step().unwrap();
        }
        assert_eq!(f.request(a).unwrap().admission_sequence, seq);
        assert_eq!(
            f.resource(r).unwrap().queued,
            if rekey { vec![a, b, c] } else { vec![b, c, a] }
        );
        let mut holder = h;
        for next in if rekey { vec![a, b, c] } else { vec![b, c, a] } {
            f.release(f.request(holder).unwrap().lease.unwrap(), t(1))
                .unwrap();
            f.step().unwrap();
            assert_eq!(f.request(next).unwrap().state, RequestState::Active);
            holder = next;
        }
    }
}
#[test]
fn deadline_expires_before_release_in_both_insertion_orders() {
    for release_first in [false, true] {
        let mut f = FlowRuntime::new();
        let o = f.spawn_actor().unwrap();
        let r = f.create_resource(1).unwrap();
        let h = f.submit(r, o, t(0)).unwrap();
        f.step().unwrap();
        let lease = f.request(h).unwrap().lease.unwrap();
        if release_first {
            f.release(lease, t(5)).unwrap();
        }
        let q = f.acquire(r).owner(o).deadline(t(5)).submit().unwrap();
        f.step().unwrap();
        if !release_first {
            f.release(lease, t(5)).unwrap();
        }
        let run = f.run_for(10).unwrap();
        assert_eq!(f.request(q).unwrap().state, RequestState::TimedOut);
        assert_eq!(f.request(h).unwrap().state, RequestState::Released);
        assert_eq!(
            run.dispatches
                .iter()
                .flat_map(|d| &d.records)
                .filter(|e| e.request == q && e.state == RequestState::TimedOut)
                .count(),
            1
        );
        assert!(!run
            .dispatches
            .iter()
            .flat_map(|d| &d.records)
            .any(|e| e.request == q && e.state == RequestState::Active));
    }
}
#[test]
fn grant_clears_deadline_and_immediate_expiry_never_grants() {
    let mut f = FlowRuntime::new();
    let o = f.spawn_actor().unwrap();
    let r = f.create_resource(1).unwrap();
    let q = f.acquire(r).owner(o).deadline(t(5)).submit().unwrap();
    f.run_for(10).unwrap();
    assert_eq!(f.request(q).unwrap().state, RequestState::Active);
    assert_eq!(f.request(q).unwrap().deadline, None);
    let r2 = f.create_resource(1).unwrap();
    let q2 = f.acquire(r2).owner(o).deadline(t(4)).submit().unwrap();
    f.run_for(10).unwrap();
    assert_eq!(f.request(q2).unwrap().state, RequestState::TimedOut);
    assert!(f.resource(r2).unwrap().active.is_empty());
}
#[test]
fn cancellation_and_rekey_follow_scheduler_order_and_resubmission_is_new() {
    for rekey_first in [false, true] {
        let mut f = FlowRuntime::new();
        let o = f.spawn_actor().unwrap();
        let r = f.create_resource(0).unwrap();
        let a = f.submit(r, o, t(0)).unwrap();
        let b = f.submit(r, o, t(0)).unwrap();
        f.run_for(2).unwrap();
        f.cancel(a, t(1)).unwrap();
        f.reprioritize_with_scheduler_priority(a, -1, t(1), if rekey_first { -1 } else { 0 })
            .unwrap();
        let run = f.run_for(2).unwrap();
        assert_eq!(f.request(a).unwrap().state, RequestState::Cancelled);
        assert_eq!(
            run.dispatches[1].error,
            if rekey_first {
                None
            } else {
                Some(FlowError::TerminalRequest)
            }
        );
        let a2 = f.submit(r, o, t(1)).unwrap();
        f.step().unwrap();
        assert_ne!(a, a2);
        assert_eq!(f.resource(r).unwrap().queued, vec![b, a2]);
    }
}
#[test]
fn timeout_evidence_survives_rejected_rekey_at_boundary() {
    let mut f = FlowRuntime::new();
    let o = f.spawn_actor().unwrap();
    let r = f.create_resource(0).unwrap();
    let q = f.acquire(r).owner(o).deadline(t(5)).submit().unwrap();
    f.step().unwrap();
    f.reprioritize_with_scheduler_priority(q, -1, t(5), -1)
        .unwrap();
    let d = f.step().unwrap().unwrap();
    assert_eq!(d.error, Some(FlowError::TerminalRequest));
    assert_eq!(d.records.len(), 1);
    assert_eq!(d.records[0].state, RequestState::TimedOut);
    assert_eq!(f.request(q).unwrap().state, RequestState::TimedOut);
    assert!(f.step().unwrap().unwrap().records.is_empty());
}
