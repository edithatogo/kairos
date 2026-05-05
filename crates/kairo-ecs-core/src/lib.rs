#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

use kairo_ecs_types::{DispatchedEvent, EventId, ScheduleRequest, SimTime, StepOutcome};

#[derive(Clone, Debug, Eq, PartialEq)]
struct QueueEntry {
    request: ScheduleRequest,
    sequence: u64,
    id: EventId,
}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .request
            .at
            .cmp(&self.request.at)
            .then_with(|| other.request.priority.cmp(&self.request.priority))
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Default seed for Scheduler when created via Default trait
const DEFAULT_SEED: u64 = 0;

/// Deterministic single-threaded scheduler.
#[derive(Debug, Default)]
pub struct Scheduler {
    heap: BinaryHeap<QueueEntry>,
    cancelled: HashSet<EventId>,
    next_event_index: u64,
    next_event_generation: u32,
    next_sequence: u64,
    now: SimTime,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn now(&self) -> SimTime {
        self.now
    }

    pub fn schedule(&mut self, request: ScheduleRequest) -> EventId {
        let id = EventId {
            index: self.next_event_index,
            generation: self.next_event_generation,
        };
        self.next_event_index += 1;
        self.next_event_generation = self.next_event_generation.wrapping_add(1);

        let entry = QueueEntry {
            request,
            sequence: self.next_sequence,
            id,
        };
        self.next_sequence += 1;
        self.heap.push(entry);
        id
    }

    pub fn cancel(&mut self, id: EventId) -> bool {
        if self.cancelled.contains(&id) || !self.heap.iter().any(|entry| entry.id == id) {
            return false;
        }

        self.cancelled.insert(id)
    }

    pub fn pending_events(&self) -> usize {
        self.heap
            .iter()
            .filter(|entry| !self.cancelled.contains(&entry.id))
            .count()
    }

    fn next_event_at(&self) -> Option<SimTime> {
        self.heap
            .iter()
            .filter(|entry| !self.cancelled.contains(&entry.id))
            .map(|entry| entry.request.at)
            .min()
    }

    pub fn step(&mut self) -> StepOutcome {
        while let Some(entry) = self.heap.pop() {
            if self.cancelled.remove(&entry.id) {
                continue;
            }

            self.now = entry.request.at;
            return StepOutcome::Dispatched(DispatchedEvent {
                id: entry.id,
                at: entry.request.at,
                priority: entry.request.priority,
                sequence: entry.sequence,
                entity: entry.request.entity,
                kind: entry.request.kind,
            });
        }

        StepOutcome::Empty
    }

    pub fn run_for(&mut self, max_events: u64) -> StepOutcome {
        if max_events == 0 {
            return StepOutcome::LimitReached;
        }

        let mut last = StepOutcome::Empty;
        for _ in 0..max_events {
            match self.step() {
                StepOutcome::Dispatched(event) => {
                    last = StepOutcome::Dispatched(event);
                }
                StepOutcome::Empty => {
                    return if matches!(last, StepOutcome::Empty) {
                        StepOutcome::Empty
                    } else {
                        last
                    };
                }
                StepOutcome::LimitReached => return StepOutcome::LimitReached,
            }
        }

        if self.pending_events() == 0 {
            if matches!(last, StepOutcome::Empty) {
                StepOutcome::Empty
            } else {
                last
            }
        } else {
            StepOutcome::LimitReached
        }
    }

    pub fn run_until(&mut self, time_limit: SimTime) -> StepOutcome {
        let mut last = StepOutcome::Empty;

        while let Some(next_at) = self.next_event_at() {
            if next_at > time_limit {
                return if self.heap.is_empty() {
                    StepOutcome::Empty
                } else {
                    StepOutcome::LimitReached
                };
            }

            match self.step() {
                StepOutcome::Dispatched(event) => last = StepOutcome::Dispatched(event),
                StepOutcome::Empty => return last,
                StepOutcome::LimitReached => return StepOutcome::LimitReached,
            }
        }

        last
    }

    pub fn run_until_or_for(&mut self, time_limit: SimTime, max_events: u64) -> StepOutcome {
        if max_events == 0 {
            return StepOutcome::LimitReached;
        }

        let mut last = StepOutcome::Empty;
        for _ in 0..max_events {
            match self.next_event_at() {
                Some(next_at) if next_at > time_limit => {
                    return if self.pending_events() == 0 {
                        last
                    } else {
                        StepOutcome::LimitReached
                    };
                }
                Some(_) => match self.step() {
                    StepOutcome::Dispatched(event) => last = StepOutcome::Dispatched(event),
                    StepOutcome::Empty => return last,
                    StepOutcome::LimitReached => return StepOutcome::LimitReached,
                },
                None => return last,
            }
        }

        if self.pending_events() == 0 {
            last
        } else {
            StepOutcome::LimitReached
        }
    }
}

#[cfg(test)]
mod tests {
    use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime, StepOutcome};

    use super::*;

    fn request(at: u128, priority: i32, kind: u32) -> ScheduleRequest {
        ScheduleRequest {
            at: SimTime::from_ticks(at),
            priority,
            entity: None,
            kind: EventKind::Custom(kind),
        }
    }

    #[test]
    fn dispatches_by_time_priority_then_sequence() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(request(10, 2, 3));
        scheduler.schedule(request(5, 9, 1));
        scheduler.schedule(request(10, 1, 2));
        scheduler.schedule(request(10, 1, 4));

        let mut kinds = Vec::new();
        while let StepOutcome::Dispatched(event) = scheduler.step() {
            let EventKind::Custom(kind) = event.kind;
            kinds.push(kind);
        }

        assert_eq!(kinds, vec![1, 2, 4, 3]);
    }

    #[test]
    fn cancellation_skips_event_without_reordering_rest() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(request(1, 0, 1));
        let cancelled = scheduler.schedule(request(2, 0, 2));
        scheduler.schedule(request(3, 0, 3));

        scheduler.cancel(cancelled);
        assert!(!scheduler.cancel(cancelled));
        assert_eq!(scheduler.pending_events(), 2);

        let first = scheduler.step();
        let second = scheduler.step();
        let third = scheduler.step();

        assert!(matches!(first, StepOutcome::Dispatched(_)));
        assert!(matches!(second, StepOutcome::Dispatched(_)));
        assert_eq!(third, StepOutcome::Empty);
        assert_eq!(scheduler.pending_events(), 0);
    }

    #[test]
    fn cancellation_rejects_unknown_and_dispatched_events() {
        let mut scheduler = Scheduler::new();
        let dispatched = scheduler.schedule(request(1, 0, 1));

        assert!(matches!(scheduler.step(), StepOutcome::Dispatched(_)));
        assert!(!scheduler.cancel(dispatched));
        assert!(!scheduler.cancel(EventId {
            index: 999,
            generation: 0,
        }));
        assert_eq!(scheduler.pending_events(), 0);
    }

    #[test]
    fn cancelled_future_event_does_not_force_limit_after_active_events_finish() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(request(1, 0, 1));
        let cancelled = scheduler.schedule(request(100, 0, 2));
        assert!(scheduler.cancel(cancelled));

        let outcome = scheduler.run_for(8);

        assert!(matches!(outcome, StepOutcome::Dispatched(_)));
        assert_eq!(scheduler.pending_events(), 0);
        assert_eq!(scheduler.step(), StepOutcome::Empty);
    }

    #[test]
    fn run_for_is_bounded() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(request(1, 0, 1));
        scheduler.schedule(request(2, 0, 2));
        scheduler.schedule(request(3, 0, 3));

        let outcome = scheduler.run_for(2);
        assert!(matches!(outcome, StepOutcome::Dispatched(_)));
        assert_eq!(scheduler.pending_events(), 1);
    }

    #[test]
    fn run_for_with_zero_limit_reports_limit_reached() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(request(1, 0, 1));

        assert_eq!(scheduler.run_for(0), StepOutcome::LimitReached);
        assert_eq!(scheduler.pending_events(), 1);
    }

    #[test]
    fn run_until_stops_at_time_limit() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(request(1, 0, 1));
        scheduler.schedule(request(4, 0, 2));

        let outcome = scheduler.run_until(SimTime::from_ticks(1));
        assert!(matches!(outcome, StepOutcome::Dispatched(_)));
        assert_eq!(scheduler.now(), SimTime::from_ticks(1));
        assert_eq!(scheduler.pending_events(), 1);
    }

    #[test]
    fn run_until_or_for_stops_on_bounds() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(request(1, 0, 1));
        scheduler.schedule(request(2, 0, 2));

        let outcome = scheduler.run_until_or_for(SimTime::from_ticks(1), 1);
        assert!(matches!(outcome, StepOutcome::Dispatched(_)));
        assert_eq!(scheduler.pending_events(), 1);

        let bounded = scheduler.run_until_or_for(SimTime::from_ticks(0), 0);
        assert_eq!(bounded, StepOutcome::LimitReached);
    }
}
