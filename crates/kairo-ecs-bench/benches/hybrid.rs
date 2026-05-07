use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use kairo_ecs_bench::{HYBRID_BENCH_SCALE, PRIORITY_RANGE};
use kairo_ecs_core::Scheduler;
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_state::World;
use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime};

fn bench_hybrid_des_abm_smoke_100k(c: &mut Criterion) {
    let scale = HYBRID_BENCH_SCALE;
    let mut group = c.benchmark_group("hybrid_des_abm_smoke_100k");
    group.throughput(Throughput::Elements(scale));
    group.bench_function("schedule_and_pop", |b| {
        b.iter_batched(
            || {
                let mut scheduler = Scheduler::new();
                let mut world = World::new();
                let mut rng = DeterministicStream::new(42);
                for index in 0..scale {
                    let entity = world.spawn();
                    let priority = (rng.next_u64() % PRIORITY_RANGE) as i32;
                    scheduler.schedule(ScheduleRequest {
                        at: SimTime::from_ticks((index % 1000) as u128),
                        priority,
                        entity: Some(entity),
                        kind: EventKind::Custom(index as u32),
                    });
                }
                scheduler
            },
            |mut scheduler| {
                black_box(scheduler.run_for(scale));
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

criterion_group!(hybrid_benches, bench_hybrid_des_abm_smoke_100k);
criterion_main!(hybrid_benches);
