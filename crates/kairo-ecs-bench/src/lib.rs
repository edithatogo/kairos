#![forbid(unsafe_code)]

use kairo_ecs_core::Scheduler;
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_state::World;
use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime, StepOutcome};

/// Default benchmark scale for the canonical 1M-event/entity scenarios.
pub const BENCH_SCALE: u64 = 1_000_000;

/// Smoke-test scale used by the metadata-only benchmark harness.
pub const SMOKE_SCALE: u64 = 4;

/// Scale for the mixed DES/ABM smoke benchmark.
pub const HYBRID_BENCH_SCALE: u64 = 100_000;

/// Priority range modulus used by the scheduler smoke helper.
pub const PRIORITY_RANGE: u64 = 11;

/// Seed-derived priority modulus used by the hybrid smoke helper.
pub const SEED_PRIORITY_MOD: u64 = 7;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BenchmarkScenario {
    pub id: &'static str,
    pub scale: u64,
    pub smoke_scale: u64,
    pub requires_native_link_tests: bool,
}

impl BenchmarkScenario {
    pub const fn new(
        id: &'static str,
        scale: u64,
        smoke_scale: u64,
        requires_native_link_tests: bool,
    ) -> Self {
        Self {
            id,
            scale,
            smoke_scale,
            requires_native_link_tests,
        }
    }
}

pub const CANONICAL_BENCHMARK_SCENARIOS: [BenchmarkScenario; 6] = [
    BenchmarkScenario::new("schedule_1m_events", BENCH_SCALE, SMOKE_SCALE, false),
    BenchmarkScenario::new("pop_1m_events", BENCH_SCALE, SMOKE_SCALE, false),
    BenchmarkScenario::new("schedule_cancel_1m_mixed", BENCH_SCALE, SMOKE_SCALE, false),
    BenchmarkScenario::new("create_1m_entities", BENCH_SCALE, SMOKE_SCALE, false),
    BenchmarkScenario::new("component_insert_1m", BENCH_SCALE, SMOKE_SCALE, false),
    BenchmarkScenario::new(
        "hybrid_des_abm_smoke_100k",
        HYBRID_BENCH_SCALE,
        SMOKE_SCALE,
        false,
    ),
];

pub const BENCHMARK_SCENARIOS: &[BenchmarkScenario] = &CANONICAL_BENCHMARK_SCENARIOS;

pub const BENCHMARK_SCENARIO_IDS: [&str; 6] = [
    "schedule_1m_events",
    "pop_1m_events",
    "schedule_cancel_1m_mixed",
    "create_1m_entities",
    "component_insert_1m",
    "hybrid_des_abm_smoke_100k",
];

pub fn schedule_1m_events_smoke(seed: u64, count: u64) -> StepOutcome {
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

pub fn schedule_1m_events_preview(seed: u64, count: u64) -> StepOutcome {
    schedule_1m_events_smoke(seed, count)
}

pub fn hybrid_des_abm_smoke(seed: u64) -> StepOutcome {
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

pub fn hybrid_des_abm_smoke_preview(seed: u64) -> StepOutcome {
    hybrid_des_abm_smoke(seed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_scenario_metadata_is_stable() {
        assert_eq!(BENCHMARK_SCENARIOS.len(), BENCHMARK_SCENARIO_IDS.len());
        assert_eq!(
            BENCHMARK_SCENARIO_IDS,
            [
                "schedule_1m_events",
                "pop_1m_events",
                "schedule_cancel_1m_mixed",
                "create_1m_entities",
                "component_insert_1m",
                "hybrid_des_abm_smoke_100k",
            ]
        );
        assert_eq!(
            BENCHMARK_SCENARIOS,
            CANONICAL_BENCHMARK_SCENARIOS.as_slice()
        );
        assert!(BENCHMARK_SCENARIOS.iter().all(|scenario| {
            !scenario.requires_native_link_tests
                && scenario.scale > 0
                && scenario.smoke_scale > 0
                && scenario.smoke_scale <= scenario.scale
        }));
        assert_eq!(BENCHMARK_SCENARIOS[0].scale, BENCH_SCALE);
        assert_eq!(BENCHMARK_SCENARIOS[5].scale, HYBRID_BENCH_SCALE);
    }

    #[test]
    fn smoke_helpers_are_aliases_and_callable() {
        let schedule_preview = schedule_1m_events_preview(7, 4);
        let schedule_smoke = schedule_1m_events_smoke(7, 4);
        assert_eq!(schedule_preview, schedule_smoke);
        assert!(matches!(
            schedule_smoke,
            StepOutcome::LimitReached | StepOutcome::Dispatched(_) | StepOutcome::Empty
        ));

        let hybrid_preview = hybrid_des_abm_smoke_preview(7);
        let hybrid_smoke = hybrid_des_abm_smoke(7);
        assert_eq!(hybrid_preview, hybrid_smoke);
        assert!(matches!(
            hybrid_smoke,
            StepOutcome::Dispatched(_) | StepOutcome::Empty | StepOutcome::LimitReached
        ));
    }
}
