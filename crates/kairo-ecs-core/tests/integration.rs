use kairo_ecs_core::Scheduler;
use kairo_ecs_types::{DispatchedEvent, EventKind, ScheduleRequest, SimTime, StepOutcome};
use std::collections::HashSet;

fn request(at: u128, priority: i32, kind: u32) -> ScheduleRequest {
    ScheduleRequest {
        at: SimTime::from_ticks(at),
        priority,
        entity: None,
        kind: EventKind::Custom(kind),
    }
}

// ---------------------------------------------------------------------------
// 1. deterministic replay
// ---------------------------------------------------------------------------
#[test]
fn test_deterministic_replay() {
    let events = &[
        (10, 2, 3),
        (5, 9, 1),
        (10, 1, 2),
        (10, 1, 4),
        (20, 0, 5),
        (0, 10, 6),
    ];

    let collect = |s: &mut Scheduler| -> Vec<u32> {
        let mut kinds = Vec::new();
        loop {
            match s.step() {
                StepOutcome::Dispatched(DispatchedEvent {
                    kind: EventKind::Custom(k),
                    ..
                }) => kinds.push(k),
                StepOutcome::Empty => break,
                StepOutcome::LimitReached => break,
            }
        }
        kinds
    };

    let mut a = Scheduler::new();
    let mut b = Scheduler::new();

    for &(at, pri, kind) in events {
        a.schedule(request(at, pri, kind));
        b.schedule(request(at, pri, kind));
    }

    let order_a = collect(&mut a);
    let order_b = collect(&mut b);

    assert_eq!(
        order_a, order_b,
        "same events must dispatch in identical order"
    );
    assert!(!order_a.is_empty());
}

// ---------------------------------------------------------------------------
// 2. zero-delay guard — same time, different priorities
// ---------------------------------------------------------------------------
#[test]
fn test_zero_delay_guard() {
    let mut s = Scheduler::new();

    // All at tick 0, lower priority value = dispatched first
    s.schedule(request(0, 5, 5));
    s.schedule(request(0, 0, 1));
    s.schedule(request(0, 10, 10));
    s.schedule(request(0, 0, 2));

    let mut kinds = Vec::new();
    while let StepOutcome::Dispatched(e) = s.step() {
        let EventKind::Custom(k) = e.kind;
        kinds.push(k);
    }

    // priority 0 first (kind 1 then 2 by sequence), then priority 5, then 10
    assert_eq!(kinds, vec![1, 2, 5, 10]);
}

// ---------------------------------------------------------------------------
// 3. run_until stops exactly at time limit
// ---------------------------------------------------------------------------
#[test]
fn test_run_until_bounds() {
    let mut s = Scheduler::new();

    s.schedule(request(1, 0, 1));
    s.schedule(request(3, 0, 2));
    s.schedule(request(5, 0, 3));
    s.schedule(request(7, 0, 4));

    // run_until(3) should dispatch events at 1 and 3, then stop
    let outcome = s.run_until(SimTime::from_ticks(3));
    assert!(
        matches!(outcome, StepOutcome::Dispatched(_)),
        "last outcome should be Dispatched, got {:?}",
        outcome
    );

    // Remaining events at 5 and 7
    assert_eq!(s.pending_events(), 2);

    // Now time on the scheduler should be at the last dispatched event
    assert_eq!(s.now(), SimTime::from_ticks(3));

    // run_until(10) dispatches the rest
    let final_outcome = s.run_until(SimTime::from_ticks(10));
    assert!(matches!(final_outcome, StepOutcome::Dispatched(_)));
    assert_eq!(s.pending_events(), 0);
    assert_eq!(s.step(), StepOutcome::Empty);
}

// ---------------------------------------------------------------------------
// 4. run_for dispatches exactly max_events
// ---------------------------------------------------------------------------
#[test]
fn test_run_for_bounds() {
    let mut s = Scheduler::new();

    for i in 0..10u32 {
        s.schedule(request(i as u128, 0, i));
    }

    assert_eq!(s.pending_events(), 10);

    // Dispatch 4
    let outcome = s.run_for(4);
    assert!(matches!(outcome, StepOutcome::LimitReached));
    assert_eq!(s.pending_events(), 6);

    // Dispatch 6 more — queue empties
    let outcome = s.run_for(6);
    // After dispatching exactly the last event, run_for returns Dispatched (not LimitReached)
    // because pending_events() == 0
    assert!(
        matches!(outcome, StepOutcome::Dispatched(_)),
        "expected Dispatched, got {:?}",
        outcome
    );
    assert_eq!(s.pending_events(), 0);

    // run_for on empty scheduler
    assert_eq!(s.run_for(1), StepOutcome::Empty);
}

// ---------------------------------------------------------------------------
// 5. cancel then schedule
// ---------------------------------------------------------------------------
#[test]
fn test_cancel_then_schedule() {
    let mut s = Scheduler::new();

    s.schedule(request(1, 0, 1));
    let b = s.schedule(request(2, 0, 2));
    s.schedule(request(3, 0, 3));

    assert_eq!(s.pending_events(), 3);

    // Cancel the middle event
    assert!(s.cancel(b));
    assert_eq!(s.pending_events(), 2);

    // Double cancel is a no-op
    assert!(!s.cancel(b));

    // Schedule a replacement at the same time as the cancelled event
    s.schedule(request(2, 0, 4));

    // Dispatch all — order should be 1, 4, 3 (b was cancelled, d replaces)
    let mut kinds = Vec::new();
    while let StepOutcome::Dispatched(e) = s.step() {
        let EventKind::Custom(k) = e.kind;
        kinds.push(k);
    }

    assert_eq!(kinds, vec![1, 4, 3]);
}

// ---------------------------------------------------------------------------
// 6. empty scheduler
// ---------------------------------------------------------------------------
#[test]
fn test_empty_scheduler() {
    let mut s = Scheduler::new();

    assert_eq!(s.pending_events(), 0);
    assert_eq!(s.step(), StepOutcome::Empty);
    assert_eq!(s.run_for(5), StepOutcome::Empty);
    assert_eq!(s.run_until(SimTime::from_ticks(100)), StepOutcome::Empty);
    assert_eq!(
        s.run_until_or_for(SimTime::from_ticks(50), 10),
        StepOutcome::Empty
    );
}

// ---------------------------------------------------------------------------
// 7. trace recorder facade
// ---------------------------------------------------------------------------
#[test]
fn test_trace_recorder() {
    let mut s = kairo_ecs_core::RecordingScheduler::new(0);
    s.schedule(request(1, 0, 10));
    s.schedule(request(2, 1, 20));
    s.schedule(request(3, 0, 30));

    assert_eq!(s.run_for(3), 3);

    let events = &s.recorded;
    assert_eq!(events.len(), 3);

    for w in events.windows(2) {
        assert!(w[0].tick <= w[1].tick);
    }

    let kinds: Vec<u32> = events.iter().map(|e| e.kind).collect();
    assert_eq!(kinds, vec![10, 20, 30]);
}

// ---------------------------------------------------------------------------
// 8. large event count – 10 000 events, random times/priorities
// ---------------------------------------------------------------------------
#[test]
fn test_large_event_count() {
    // Simple LCG (same as glibc / ANSI C): X_{n+1} = (1103515245 * X_n + 12345) % 2^31
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12_345) & 0x7FFF_FFFF;
            self.0
        }

        fn next_in_range(&mut self, lo: u64, hi: u64) -> u64 {
            lo + (self.next() % (hi - lo))
        }
    }

    let mut s = Scheduler::new();
    let mut rng = Lcg(0xCAFE_BABE);

    let count = 10_000u32;

    for kind in 0..count {
        let at = rng.next_in_range(0, 50_000) as u128;
        let priority = rng.next_in_range(0, 100) as i32;
        s.schedule(request(at, priority, kind));
    }

    assert_eq!(s.pending_events(), count as usize);

    let mut dispatched = 0u32;
    let mut seen: HashSet<u32> = HashSet::with_capacity(count as usize);
    let mut prev: Option<(u128, i32, u64)> = None;

    while let StepOutcome::Dispatched(event) = s.step() {
        dispatched += 1;

        // Verify no duplicate event kinds
        let EventKind::Custom(kind) = event.kind;
        assert!(
            seen.insert(kind),
            "duplicate event kind {} dispatched",
            kind
        );

        // Verify ordering: (at, priority, sequence) is non-decreasing lexicographically
        let current = (event.at.ticks(), event.priority, event.sequence);
        if let Some(prev_tuple) = prev {
            assert!(
                prev_tuple <= current,
                "ordering violation: {:?} > {:?}",
                prev_tuple,
                current
            );
        }
        prev = Some(current);
    }

    assert_eq!(dispatched, count, "must dispatch all {} events", count);
    assert_eq!(s.pending_events(), 0);
    assert_eq!(s.step(), StepOutcome::Empty);
}
