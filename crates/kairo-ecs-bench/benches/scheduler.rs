use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use kairo_ecs_bench::{BENCH_SCALE, PRIORITY_RANGE};
use kairo_ecs_core::Scheduler;
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_state::World;
use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime};

fn bench_schedule_1m(c: &mut Criterion) {
    let scale = BENCH_SCALE;
    let mut group = c.benchmark_group("schedule_1m_events");
    group.throughput(Throughput::Elements(scale));
    group.bench_function("schedule", |b| {
        b.iter_batched(
            || {
                let mut world = World::new();
                let mut rng = DeterministicStream::new(42);
                let mut requests = Vec::with_capacity(scale as usize);
                for index in 0..scale {
                    let entity = world.spawn();
                    let priority = (rng.next_u64() % PRIORITY_RANGE) as i32;
                    requests.push(ScheduleRequest {
                        at: SimTime::from_ticks(index as u128),
                        priority,
                        entity: Some(entity),
                        kind: EventKind::Custom(index as u32),
                    });
                }
                (requests, Scheduler::new())
            },
            |(requests, mut scheduler)| {
                for req in requests {
                    black_box(scheduler.schedule(req));
                }
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

fn bench_pop_1m(c: &mut Criterion) {
    let scale = BENCH_SCALE;
    let mut group = c.benchmark_group("pop_1m_events");
    group.throughput(Throughput::Elements(scale));
    group.bench_function("pop", |b| {
        b.iter_batched(
            || {
                let mut scheduler = Scheduler::new();
                let mut world = World::new();
                let mut rng = DeterministicStream::new(42);
                for index in 0..scale {
                    let entity = world.spawn();
                    let priority = (rng.next_u64() % PRIORITY_RANGE) as i32;
                    scheduler.schedule(ScheduleRequest {
                        at: SimTime::from_ticks(index as u128),
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

fn bench_schedule_cancel_mixed(c: &mut Criterion) {
    let scale = BENCH_SCALE;
    let mut group = c.benchmark_group("schedule_cancel_1m_mixed");
    group.throughput(Throughput::Elements(scale));
    group.bench_function("schedule_cancel_pop", |b| {
        b.iter_batched(
            || {
                let mut scheduler = Scheduler::new();
                let mut world = World::new();
                let mut rng = DeterministicStream::new(42);
                let mut ids = Vec::with_capacity(scale as usize);
                for index in 0..scale {
                    let entity = world.spawn();
                    let priority = (rng.next_u64() % PRIORITY_RANGE) as i32;
                    let id = scheduler.schedule(ScheduleRequest {
                        at: SimTime::from_ticks(index as u128),
                        priority,
                        entity: Some(entity),
                        kind: EventKind::Custom(index as u32),
                    });
                    ids.push(id);
                }
                for entry in ids.iter().step_by(2) {
                    scheduler.cancel(*entry);
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

criterion_group!(
    scheduler_benches,
    bench_schedule_1m,
    bench_pop_1m,
    bench_schedule_cancel_mixed,
);
criterion_main!(scheduler_benches);
