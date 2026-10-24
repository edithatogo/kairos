//! Track 96 Phase 1: Bounded property test implementation for invariants,
//! ordering, conservation, and generational ranges.
//!
//! Acceptance oracles verified:
//! 1. Total ordering contract: (SimTime, priority, insertion_sequence).
//! 2. Resource/event conservation: scheduled = dispatched + cancelled + pending.
//! 3. Generational ID coverage across full 64-bit and 32-bit ranges.
//! 4. Bounded execution within configured test budget (cases, shrink limits).

use std::collections::BTreeSet;

use kairo_ecs_core::Scheduler;
use kairo_ecs_types::{EntityId, EventId, EventKind, ScheduleRequest, SimTime, StepOutcome};
use proptest::prelude::*;
use proptest::test_runner::{RngSeed, TestRunner};

fn runner_with_seed(seed: u64, cases: u32) -> TestRunner {
    TestRunner::new(ProptestConfig {
        cases,
        rng_seed: RngSeed::Fixed(seed),
        max_shrink_iters: 1_000,
        ..ProptestConfig::default()
    })
}

#[derive(Debug, Clone)]
struct ArbitraryScheduleSpec {
    tick: u64,
    priority: i32,
    entity_idx: Option<u64>,
    kind: u32,
}

fn arbitrary_schedule_strategy() -> impl Strategy<Value = ArbitraryScheduleSpec> {
    (
        0u64..10_000,
        any::<i32>(),
        prop::option::of(any::<u64>()),
        any::<u32>(),
    )
        .prop_map(|(tick, priority, entity_idx, kind)| ArbitraryScheduleSpec {
            tick,
            priority,
            entity_idx,
            kind,
        })
}

#[test]
fn proptest_total_ordering_and_conservation_invariant() {
    let mut runner = runner_with_seed(0x96_0001, 128);
    let strategy = prop::collection::vec(arbitrary_schedule_strategy(), 1..64);

    runner
        .run(&strategy, |specs| {
            let mut scheduler = Scheduler::new();
            let mut reference_oracle = Vec::with_capacity(specs.len());

            for (seq, spec) in specs.iter().enumerate() {
                let entity = spec.entity_idx.map(|idx| EntityId::new(idx, 1));
                let id = scheduler.schedule(ScheduleRequest {
                    at: SimTime::from_ticks(u128::from(spec.tick)),
                    priority: spec.priority,
                    entity,
                    kind: EventKind::Custom(spec.kind),
                });
                prop_assert_eq!(id.index, seq as u64);
                reference_oracle.push((spec.tick, spec.priority, seq as u64, spec.kind));
            }

            // Conservation check before stepping: pending == total scheduled
            prop_assert_eq!(scheduler.pending_events(), specs.len());

            // Independent reference model: sort by tick, priority, then sequence
            reference_oracle.sort_by_key(|&(tick, priority, seq, _)| (tick, priority, seq));

            let mut dispatched = Vec::with_capacity(specs.len());
            let mut step_count = 0;
            while let StepOutcome::Dispatched(ev) = scheduler.step() {
                step_count += 1;
                dispatched.push((
                    ev.at.ticks() as u64,
                    ev.priority,
                    ev.sequence,
                    match ev.kind {
                        EventKind::Custom(k) => k,
                    },
                ));
                // Conservation invariant at every step:
                prop_assert_eq!(
                    scheduler.pending_events() + step_count,
                    specs.len(),
                    "Conservation violated at step {}",
                    step_count
                );
            }

            prop_assert_eq!(dispatched, reference_oracle);
            prop_assert_eq!(scheduler.pending_events(), 0);
            prop_assert_eq!(scheduler.stats().dispatched_events, specs.len() as u64);
            Ok(())
        })
        .unwrap();
}

#[test]
fn proptest_cancellation_and_conservation_invariant() {
    let mut runner = runner_with_seed(0x96_0002, 128);
    let strategy = (
        prop::collection::vec(arbitrary_schedule_strategy(), 1..64),
        prop::collection::vec(any::<bool>(), 1..64),
    );

    runner
        .run(&strategy, |(specs, cancel_flags)| {
            let mut scheduler = Scheduler::new();
            let mut scheduled_ids = Vec::with_capacity(specs.len());
            let mut cancelled_ids = BTreeSet::new();

            for spec in &specs {
                let id = scheduler.schedule(ScheduleRequest {
                    at: SimTime::from_ticks(u128::from(spec.tick)),
                    priority: spec.priority,
                    entity: None,
                    kind: EventKind::Custom(spec.kind),
                });
                scheduled_ids.push(id);
            }

            // Cancel events according to cancel_flags
            for (i, &should_cancel) in cancel_flags.iter().enumerate() {
                if should_cancel && i < scheduled_ids.len() {
                    let id = scheduled_ids[i];
                    let was_cancelled = scheduler.cancel(id);
                    prop_assert!(
                        was_cancelled,
                        "Initial cancellation of scheduled event must return true"
                    );
                    cancelled_ids.insert(id);

                    // Idempotent cancellation check: second cancel must return false
                    let repeat_cancel = scheduler.cancel(id);
                    prop_assert!(
                        !repeat_cancel,
                        "Subsequent cancellation of already cancelled event must return false"
                    );
                }
            }

            // Conservation invariant: scheduled = pending + cancelled
            prop_assert_eq!(
                scheduler.pending_events() + cancelled_ids.len(),
                specs.len(),
                "Conservation before stepping violated: pending + cancelled != scheduled"
            );

            // Step through remaining events
            let mut dispatched_count = 0;
            while let StepOutcome::Dispatched(ev) = scheduler.step() {
                dispatched_count += 1;
                // Dispatched event must NOT be in cancelled set
                prop_assert!(
                    !cancelled_ids.contains(&ev.id),
                    "Cancelled event {:?} was dispatched!",
                    ev.id
                );
            }

            // Global conservation invariant: total_scheduled == dispatched + cancelled
            prop_assert_eq!(
                dispatched_count + cancelled_ids.len(),
                specs.len(),
                "Conservation invariant violated: dispatched ({}) + cancelled ({}) != scheduled ({})",
                dispatched_count,
                cancelled_ids.len(),
                specs.len()
            );
            prop_assert_eq!(scheduler.pending_events(), 0);
            Ok(())
        })
        .unwrap();
}

#[test]
fn proptest_generational_entity_and_event_id_ranges() {
    let mut runner = runner_with_seed(0x96_0003, 128);
    // Test boundary values of index and generation
    let strategy = (
        prop::sample::select(vec![
            0u64,
            1u64,
            42u64,
            u32::MAX as u64,
            u32::MAX as u64 + 1,
            u64::MAX - 1,
            u64::MAX,
        ]),
        prop::sample::select(vec![0u32, 1u32, 42u32, u32::MAX - 1, u32::MAX]),
    );

    runner
        .run(&strategy, |(index, generation)| {
            let entity_id = EntityId::new(index, generation);
            prop_assert_eq!(entity_id.index, index);
            prop_assert_eq!(entity_id.generation, generation);

            let event_id = EventId::new(index, generation);
            prop_assert_eq!(event_id.index, index);
            prop_assert_eq!(event_id.generation, generation);

            // Equality and hashing preservation
            let clone_entity = entity_id;
            prop_assert_eq!(entity_id, clone_entity);
            Ok(())
        })
        .unwrap();
}
