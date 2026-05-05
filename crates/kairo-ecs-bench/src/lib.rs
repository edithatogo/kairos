#![forbid(unsafe_code)]

use kairo_ecs_core::Scheduler;
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_state::World;
use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime, StepOutcome};

/// Default benchmark scale (1M events/entities)
const BENCH_SCALE: u64 = 1_000_000;

/// Smoke test scale (reduced for CI speed)
const SMOKE_SCALE: u64 = 4;

/// Hybrid benchmark scale
const HYBRID_BENCH_SCALE: u64 = 100_000;

/// Priority range modulus
const PRIORITY_RANGE: u64 = 11;

/// Seed-derived priority modulus
const SEED_PRIORITY_MOD: u64 = 7;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BenchmarkScenario {
    pub id: &'static str,
    pub scale: u64,
    pub smoke_scale: u64,
    pub requires_native_link_tests: bool,
}

pub const BENCHMARK_SCENARIOS: &[BenchmarkScenario] = &[
    BenchmarkScenario {
        id: "schedule_1m_events",
        scale: BENCH_SCALE,
        smoke_scale: SMOKE_SCALE,
        requires_native_link_tests: false,
    },
    BenchmarkScenario {
        id: "pop_1m_events",
        scale: BENCH_SCALE,
        smoke_scale: SMOKE_SCALE,
        requires_native_link_tests: false,
    },
    BenchmarkScenario {
        id: "schedule_cancel_1m_mixed",
        scale: BENCH_SCALE,
        smoke_scale: SMOKE_SCALE,
        requires_native_link_tests: false,
    },
    BenchmarkScenario {
        id: "create_1m_entities",
        scale: BENCH_SCALE,
        smoke_scale: SMOKE_SCALE,
        requires_native_link_tests: false,
    },
    BenchmarkScenario {
        id: "component_insert_1m",
        scale: BENCH_SCALE,
        smoke_scale: SMOKE_SCALE,
        requires_native_link_tests: false,
    },
    BenchmarkScenario {
        id: "hybrid_des_abm_smoke_100k",
        scale: HYBRID_BENCH_SCALE,
        smoke_scale: SMOKE_SCALE,
        requires_native_link_tests: false,
    },
];

pub fn schedule_1m_events_preview(seed: u64, count: u64) -> StepOutcome {
    let mut scheduler = Scheduler::new();
    let mut world = World::new();
    let mut rng = DeterministicStream::new(seed);

    for index in 0..count {
        let entity = world.spawn();
        let priority = (rng.next_u64() % PRIORITY_RANGE) as i32;
        scheduler.schedule(ScheduleRequest {
            at: SimTime::from_ticks(index as u128),
            priority,
            entity: Some(entity),
            kind: EventKind::Custom(index as u32),
        });
    }

    scheduler.run_for(count)
}

pub fn hybrid_des_abm_smoke_preview(seed: u64) -> StepOutcome {
    let mut scheduler = Scheduler::new();
    let mut world = World::new();
    let entity = world.spawn();
    let priority = (seed % SEED_PRIORITY_MOD) as i32;

    scheduler.schedule(ScheduleRequest {
        at: SimTime::from_ticks(1),
        priority,
        entity: Some(entity),
        kind: EventKind::Custom(1),
    });

    scheduler.step()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_helpers_are_callable() {
        assert!(matches!(
            schedule_1m_events_preview(7, 4),
            StepOutcome::LimitReached | StepOutcome::Dispatched(_) | StepOutcome::Empty
        ));
        assert!(matches!(
            hybrid_des_abm_smoke_preview(7),
            StepOutcome::Dispatched(_) | StepOutcome::Empty | StepOutcome::LimitReached
        ));
    }

    #[test]
    fn benchmark_scenario_metadata_is_canonical() {
        let ids: Vec<&str> = BENCHMARK_SCENARIOS
            .iter()
            .map(|scenario| scenario.id)
            .collect();
        assert_eq!(
            ids,
            vec![
                "schedule_1m_events",
                "pop_1m_events",
                "schedule_cancel_1m_mixed",
                "create_1m_entities",
                "component_insert_1m",
                "hybrid_des_abm_smoke_100k",
            ]
        );
        assert!(BENCHMARK_SCENARIOS
            .iter()
            .all(|scenario| !scenario.requires_native_link_tests
                && scenario.scale > 0
                && scenario.smoke_scale > 0
                && scenario.smoke_scale <= scenario.scale));
    }
}
