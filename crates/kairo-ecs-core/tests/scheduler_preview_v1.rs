use kairo_ecs_core::{ScheduledEventPreview, Scheduler, SchedulerStats};
use kairo_ecs_types::{
    DispatchedEvent, EntityId, EventId, EventKind, ScheduleRequest, SimTime, StepOutcome,
};

fn request(at: u128, priority: i32, entity: Option<EntityId>, kind: u32) -> ScheduleRequest {
    ScheduleRequest {
        at: SimTime::from_ticks(at),
        priority,
        entity,
        kind: EventKind::Custom(kind),
    }
}

fn expected(id: EventId, sequence: u64, input: ScheduleRequest) -> ScheduledEventPreview {
    ScheduledEventPreview {
        id,
        at: input.at,
        priority: input.priority,
        sequence,
        entity: input.entity,
        kind: input.kind,
    }
}

fn preview_unchanged(s: &mut Scheduler, want: Option<ScheduledEventPreview>) {
    let stats = s.stats();
    let pending = s.pending_events();
    for _ in 0..3 {
        assert_eq!(s.peek_next(), want);
        assert_eq!(s.stats(), stats, "preview changed complete scheduler stats");
        assert_eq!(s.pending_events(), pending);
        assert_eq!(s.now(), stats.now);
    }
}

fn step_exact(s: &mut Scheduler, want: ScheduledEventPreview) {
    preview_unchanged(s, Some(want));
    let before = s.stats();
    assert_eq!(
        s.step(),
        StepOutcome::Dispatched(DispatchedEvent {
            id: want.id,
            at: want.at,
            priority: want.priority,
            sequence: want.sequence,
            entity: want.entity,
            kind: want.kind,
        })
    );
    assert_eq!(
        s.stats(),
        SchedulerStats {
            now: want.at,
            scheduled_events: before.scheduled_events,
            dispatched_events: before.dispatched_events + 1,
            cancelled_events: before.cancelled_events,
            pending_events: before.pending_events - 1,
        }
    );
    assert_eq!(s.pending_events() as u64, before.pending_events - 1);
}

#[test]
fn empty_preview_preserves_all_stats() {
    let mut s = Scheduler::new();
    assert_eq!(s.stats(), SchedulerStats::default());
    preview_unchanged(&mut s, None);
    assert_eq!(s.step(), StepOutcome::Empty);
    assert_eq!(s.stats(), SchedulerStats::default());
}

#[test]
fn all_fields_and_copied_snapshot_match_exact_dispatch() {
    let mut s = Scheduler::new();
    let input = request(
        19,
        -17,
        Some(EntityId {
            index: 1_000,
            generation: 9,
        }),
        23_001,
    );
    let id = s.schedule(input);
    let want = expected(id, 0, input);
    assert_eq!(
        s.stats(),
        SchedulerStats {
            now: SimTime::ZERO,
            scheduled_events: 1,
            dispatched_events: 0,
            cancelled_events: 0,
            pending_events: 1,
        }
    );
    let owned = s.peek_next().expect("live preview");
    preview_unchanged(&mut s, Some(want));
    step_exact(&mut s, want);
    assert_eq!(owned, want, "copy remains valid after scheduler mutation");
    preview_unchanged(&mut s, None);
    assert!(!s.cancel(id));
    assert_eq!(s.stats().cancelled_events, 0);
}

#[test]
fn ordering_is_time_then_priority_then_insertion_sequence() {
    let mut s = Scheduler::new();
    let inputs = [
        request(10, i32::MAX, None, 1),
        request(10, 0, None, 2),
        request(10, i32::MIN, None, 3),
        request(10, 0, None, 4),
        request(0, i32::MAX, None, 5),
        request(u128::MAX, i32::MIN, None, 6),
    ];
    let ids: Vec<_> = inputs.iter().map(|r| s.schedule(*r)).collect();
    // Independent known order: earlier time wins, then smaller priority, then FIFO.
    for i in [4, 2, 1, 3, 0, 5] {
        step_exact(&mut s, expected(ids[i], i as u64, inputs[i]));
    }
    preview_unchanged(&mut s, None);
    assert_eq!(s.stats().scheduled_events, 6);
    assert_eq!(s.stats().dispatched_events, 6);
    assert_eq!(s.stats().cancelled_events, 0);
    assert_eq!(s.stats().now, SimTime::from_ticks(u128::MAX));
}

#[test]
fn cancelled_heads_are_pruned_without_stats_or_time_changes() {
    let mut s = Scheduler::new();
    let a = request(1, 0, None, 10);
    let b = request(2, 0, None, 11);
    let c = request(3, 0, None, 12);
    let ia = s.schedule(a);
    let ib = s.schedule(b);
    let ic = s.schedule(c);
    assert!(s.cancel(ia));
    assert!(s.cancel(ib));
    assert!(!s.cancel(ia));
    assert!(!s.cancel(EventId {
        index: 99_999,
        generation: 99
    }));
    assert_eq!(
        s.stats(),
        SchedulerStats {
            now: SimTime::ZERO,
            scheduled_events: 3,
            dispatched_events: 0,
            cancelled_events: 2,
            pending_events: 1,
        }
    );
    preview_unchanged(&mut s, Some(expected(ic, 2, c)));
    step_exact(&mut s, expected(ic, 2, c));
    preview_unchanged(&mut s, None);
    assert_eq!(s.stats().now, c.at);
}

#[test]
fn cancelled_future_only_queue_is_empty_not_limit_reached() {
    let mut s = Scheduler::new();
    let live = request(4, 0, None, 20);
    let dead = request(u128::MAX, -1, None, 21);
    let ilive = s.schedule(live);
    let idead = s.schedule(dead);
    assert!(s.cancel(idead));
    step_exact(&mut s, expected(ilive, 0, live));
    let stats = s.stats();
    preview_unchanged(&mut s, None);
    assert_eq!(s.run_for(1), StepOutcome::Empty);
    assert_eq!(s.stats(), stats);
    assert_eq!(s.stats().now, SimTime::from_ticks(4));
}

#[test]
fn preview_then_cancel_exposes_next_live_identity() {
    let mut s = Scheduler::new();
    let a = request(5, 0, None, 30);
    let b = request(6, 0, None, 31);
    let ia = s.schedule(a);
    let ib = s.schedule(b);
    preview_unchanged(&mut s, Some(expected(ia, 0, a)));
    assert!(s.cancel(ia));
    assert_eq!(s.stats().scheduled_events, 2);
    assert_eq!(s.stats().dispatched_events, 0);
    assert_eq!(s.stats().cancelled_events, 1);
    assert_eq!(s.stats().pending_events, 1);
    step_exact(&mut s, expected(ib, 1, b));
    preview_unchanged(&mut s, None);
}

#[test]
fn scheduling_can_change_preview_without_consuming_old_head() {
    let mut s = Scheduler::new();
    let old = request(8, 4, None, 40);
    let better = request(8, -4, None, 41);
    let earlier = request(7, i32::MAX, None, 42);
    let old_id = s.schedule(old);
    let old_copy = s.peek_next().expect("old preview");
    assert_eq!(old_copy, expected(old_id, 0, old));
    let better_id = s.schedule(better);
    preview_unchanged(&mut s, Some(expected(better_id, 1, better)));
    let earlier_id = s.schedule(earlier);
    preview_unchanged(&mut s, Some(expected(earlier_id, 2, earlier)));
    step_exact(&mut s, expected(earlier_id, 2, earlier));
    step_exact(&mut s, expected(better_id, 1, better));
    step_exact(&mut s, expected(old_id, 0, old));
    assert_eq!(old_copy, expected(old_id, 0, old));
    preview_unchanged(&mut s, None);
}
