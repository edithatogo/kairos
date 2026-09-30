use kairo_ecs_core::Scheduler;
use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime, StepOutcome};
use proptest::prelude::*;
use proptest::test_runner::{RngSeed, TestRunner};

fn request(at: u16, priority: i8, kind: u32) -> ScheduleRequest {
    ScheduleRequest {
        at: SimTime::from_ticks(u128::from(at)),
        priority: i32::from(priority),
        entity: None,
        kind: EventKind::Custom(kind),
    }
}

fn fixed_runner() -> TestRunner {
    TestRunner::new(ProptestConfig {
        cases: 256,
        rng_seed: RngSeed::Fixed(0x4b41_4952_4f53),
        ..ProptestConfig::default()
    })
}

#[test]
fn generated_events_follow_time_priority_and_insertion_order() {
    fixed_runner()
        .run(
            &prop::collection::vec((0u16..64, -32i8..32, any::<u32>()), 0..128),
            |events| {
                let mut scheduler = Scheduler::new();
                let mut expected = Vec::with_capacity(events.len());

                for (index, &(at, priority, kind)) in events.iter().enumerate() {
                    let id = scheduler.schedule(request(at, priority, kind));
                    prop_assert_eq!(id.index, index as u64);
                    expected.push((at, priority, index as u64, id.index));
                }

                expected.sort_by_key(|(at, priority, index, _)| (*at, *priority, *index));

                let mut observed = Vec::with_capacity(events.len());
                while let StepOutcome::Dispatched(event) = scheduler.step() {
                    observed.push((event.id.index, event.sequence));
                }

                let expected: Vec<_> = expected
                    .into_iter()
                    .map(|(_, _, index, id)| (id, index))
                    .collect();
                prop_assert_eq!(observed, expected);
                prop_assert_eq!(scheduler.pending_events(), 0);
                prop_assert_eq!(scheduler.stats().dispatched_events, events.len() as u64);
                Ok(())
            },
        )
        .expect("generated schedules must preserve the total ordering contract");
}

#[test]
fn repeated_equal_priority_events_keep_their_insertion_sequence() {
    let mut scheduler = Scheduler::new();
    for kind in 0..16 {
        scheduler.schedule(request(7, 3, kind));
    }

    let mut observed = Vec::new();
    while let StepOutcome::Dispatched(event) = scheduler.step() {
        observed.push((event.kind, event.id.index, event.sequence));
    }

    let expected: Vec<_> = (0..16)
        .map(|kind| (EventKind::Custom(kind), kind as u64, kind as u64))
        .collect();
    assert_eq!(observed, expected);
}

#[test]
fn generated_cancellations_remove_only_selected_events() {
    fixed_runner()
        .run(
            &(
                prop::collection::vec((0u16..64, -32i8..32, any::<u32>()), 0..128),
                prop::collection::vec(any::<bool>(), 0..128),
            ),
            |(events, cancel_mask)| {
                let mut scheduler = Scheduler::new();
                let mut expected = Vec::with_capacity(events.len());
                let mut cancelled = 0u64;

                for (index, &(at, priority, kind)) in events.iter().enumerate() {
                    let id = scheduler.schedule(request(at, priority, kind));
                    if cancel_mask.get(index).copied().unwrap_or(false) {
                        prop_assert!(scheduler.cancel(id));
                        prop_assert!(!scheduler.cancel(id));
                        cancelled += 1;
                    } else {
                        expected.push((at, priority, index as u64, id.index));
                    }
                }

                expected.sort_by_key(|(at, priority, index, _)| (*at, *priority, *index));

                let mut observed = Vec::with_capacity(expected.len());
                while let StepOutcome::Dispatched(event) = scheduler.step() {
                    observed.push(event.id.index);
                }

                let expected: Vec<_> = expected.into_iter().map(|(_, _, _, id)| id).collect();
                prop_assert_eq!(observed, expected);
                prop_assert_eq!(scheduler.pending_events(), 0);
                prop_assert_eq!(scheduler.stats().cancelled_events, cancelled);
                prop_assert_eq!(
                    scheduler.stats().dispatched_events,
                    events.len() as u64 - cancelled
                );
                Ok(())
            },
        )
        .expect("generated cancellations must preserve scheduler accounting");
}
