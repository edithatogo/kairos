use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use kairo_ecs_bench::BENCH_SCALE;
use kairo_ecs_state::World;

fn bench_create_1m_entities(c: &mut Criterion) {
    let scale = BENCH_SCALE;
    let mut group = c.benchmark_group("create_1m_entities");
    group.throughput(Throughput::Elements(scale));
    group.bench_function("spawn", |b| {
        b.iter_batched(
            || World::new(),
            |mut world| {
                for _ in 0..scale {
                    black_box(world.spawn());
                }
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

fn bench_component_insert_1m(c: &mut Criterion) {
    let scale = BENCH_SCALE as usize;
    let mut group = c.benchmark_group("component_insert_1m");
    group.throughput(Throughput::Elements(scale as u64));
    group.bench_function("insert", |b| {
        b.iter_batched(
            || {
                let mut world = World::new();
                let mut entities = Vec::with_capacity(scale);
                for _ in 0..scale {
                    entities.push(world.spawn());
                }
                (world, entities)
            },
            |(mut world, entities)| {
                for entity in entities {
                    black_box(world.insert(entity, 0u64));
                }
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

criterion_group!(
    state_benches,
    bench_create_1m_entities,
    bench_component_insert_1m,
);
criterion_main!(state_benches);
