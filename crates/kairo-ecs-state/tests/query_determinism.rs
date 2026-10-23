//! End-to-end determinism checks driven through the public `kairo-ecs-state`
//! surface only.
//!
//! These deliberately build the same logical world twice, inserting and
//! registering in different orders, and assert identical query output. A
//! regression that leaked sparse-set dense order, registration order, or hash
//! order into an observable result would break one of these.

use kairo_ecs_state::{
    ComponentRegistry, ComponentStore, Mut, QueryAll, QueryError, Ref, TightLayout, With, Without,
    World,
};
use kairo_ecs_types::EntityId;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Position {
    x: i64,
    y: i64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Velocity {
    dx: i64,
    dy: i64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Health(i32);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Frozen;

const COUNT: u64 = 48;

fn spawn_all(world: &mut World) -> Vec<EntityId> {
    (0..COUNT).map(|_| world.spawn()).collect()
}

type Seed = (EntityId, i64, i64, i32);

fn seeds(entities: &[EntityId]) -> Vec<Seed> {
    entities
        .iter()
        .enumerate()
        .map(|(offset, &entity)| {
            let index = entity.index as i64;
            (entity, index, -index, (offset % 7) as i32)
        })
        .collect()
}

fn has_frozen(entity: EntityId) -> bool {
    entity.index.is_multiple_of(5)
}

/// Builds one logical world. `reverse` flips both the entity order and the
/// per-entity component order, so any order-sensitive leak shows up.
fn build(seeds: &[Seed], reverse: bool) -> ComponentRegistry {
    let mut registry = ComponentRegistry::new();
    if reverse {
        for &(entity, x, vx, health) in seeds {
            assert!(registry.insert(entity, Health(health)));
            assert!(registry.insert(entity, Velocity { dx: vx, dy: vx }));
            assert!(registry.insert(entity, Position { x, y: 0 }));
            if has_frozen(entity) {
                assert!(registry.insert(entity, Frozen));
            }
        }
    } else {
        for &(entity, x, vx, health) in seeds {
            if has_frozen(entity) {
                assert!(registry.insert(entity, Frozen));
            }
            assert!(registry.insert(entity, Position { x, y: 0 }));
            assert!(registry.insert(entity, Velocity { dx: vx, dy: vx }));
            assert!(registry.insert(entity, Health(health)));
        }
    }
    registry
}

fn read_rows(registry: &ComponentRegistry) -> Vec<(EntityId, i64, i64, i32)> {
    let plan = registry
        .query::<(&Ref<Position>, &Ref<Velocity>, &Ref<Health>), QueryAll>()
        .expect("plan builds");
    let mut rows = Vec::new();
    registry.for_each3(
        &plan,
        |entity, position: &Position, velocity: &Velocity, health: &Health| {
            rows.push((entity, position.x, velocity.dx, health.0));
        },
    );
    rows
}

#[test]
fn query_output_is_identical_across_insertion_and_registration_order() {
    let mut world = World::new();
    let entities = spawn_all(&mut world);
    let seeds = seeds(&entities);

    let forward = build(&seeds, false);
    let reversed = build(&seeds, true);

    let forward_rows = read_rows(&forward);
    assert_eq!(forward_rows, read_rows(&reversed));
    assert_eq!(forward_rows.len(), COUNT as usize);

    // Both registries hold the same four component types, registered in
    // different orders.
    assert_eq!(
        forward.registered_types().count(),
        reversed.registered_types().count()
    );
    assert_eq!(forward.registered_types().count(), 4);
}

#[test]
fn query_order_is_strictly_ascending_by_index() {
    let mut world = World::new();
    let entities = spawn_all(&mut world);
    let registry = build(&seeds(&entities), false);

    let plan = registry
        .query::<(&Ref<Position>, &Ref<Velocity>), (With<Health>, Without<Frozen>)>()
        .expect("plan builds");

    assert!(
        plan.entities()
            .windows(2)
            .all(|pair| pair[0].index < pair[1].index),
        "query order must be strictly ascending by entity index"
    );
    assert!(!plan.is_empty());
}

#[test]
fn index_reuse_after_despawn_does_not_disturb_query_order() {
    let mut world = World::new();
    let mut registry = ComponentRegistry::new();

    let entities = spawn_all(&mut world);
    for &entity in &entities {
        assert!(registry.insert(
            entity,
            Position {
                x: entity.index as i64,
                y: 0,
            },
        ));
        assert!(registry.insert(
            entity,
            Velocity {
                dx: entity.index as i64,
                dy: 0,
            },
        ));
    }

    let mut expected: Vec<EntityId> = entities.clone();

    // Despawn and immediately reuse every third index, twice over, so the
    // free list recycles slots whose generations have already advanced.
    for _ in 0..2 {
        for position in (0..entities.len()).step_by(3) {
            let current = expected[position];
            assert!(registry.despawn(&mut world, current));
            let reborn = world.spawn();
            assert_eq!(reborn.index, current.index);
            assert!(reborn.generation > current.generation);
            assert!(registry.insert(reborn, Position { x: 1000, y: 0 }));
            assert!(registry.insert(reborn, Velocity { dx: 1000, dy: 0 }));
            expected[position] = reborn;
        }
    }

    expected.sort_unstable_by_key(|entity| (entity.index, entity.generation));

    let plan = registry
        .query::<(&Ref<Position>, &Ref<Velocity>), QueryAll>()
        .expect("plan builds");
    assert_eq!(plan.entities(), expected.as_slice());

    let mut observed = Vec::new();
    registry.for_each2(&plan, |entity, _: &Position, _: &Velocity| {
        observed.push(entity);
    });
    assert_eq!(observed, expected);

    // Every stale handle must stay invisible to every accessor.
    for &stale in &entities {
        if world.is_alive(stale) {
            continue;
        }
        assert_eq!(registry.get::<Position>(stale), None);
        assert!(registry.component_types(stale).is_empty());
    }
}

#[test]
fn archetypes_partition_entities_reproducibly() {
    let mut world = World::new();
    let entities = spawn_all(&mut world);
    let registry = build(&seeds(&entities), true);

    let groups = registry.archetypes();
    assert_eq!(registry.archetypes(), groups);
    assert!(!groups.is_empty());

    // Membership must be a partition of the entities that carry components.
    let mut flattened: Vec<EntityId> = groups
        .iter()
        .flat_map(|group| group.entities().iter().copied())
        .collect();
    flattened.sort_unstable_by_key(|entity| (entity.index, entity.generation));
    flattened.dedup();
    assert_eq!(flattened.len(), COUNT as usize);

    // Each group reports strictly ascending component ordinals.
    for group in &groups {
        assert!(
            group.components().windows(2).all(|pair| pair[0] < pair[1]),
            "archetype components must be strictly ascending"
        );
        assert!(!group.is_empty());
        // Group ids are assigned in the same order the groups are returned.
        assert_eq!(
            groups.iter().filter(|g| g.id() <= group.id()).count(),
            group.id().ordinal() + 1
        );
    }
}

#[test]
fn tight_columns_are_reachable_and_equivalent_to_the_aligned_column() {
    let mut world = World::new();
    let entities = spawn_all(&mut world);
    let mut registry = ComponentRegistry::new();
    for &entity in &entities {
        assert!(registry.insert(
            entity,
            Position {
                x: entity.index as i64,
                y: 0,
            },
        ));
    }

    let aligned = registry.store::<Position>().expect("aligned column");
    let mut tight = ComponentStore::<Position, TightLayout>::new_tight();
    for &entity in &entities {
        assert!(tight.insert(
            entity,
            Position {
                x: entity.index as i64,
                y: 0,
            }
        ));
    }

    assert_eq!(aligned.column_alignment(), 64);
    assert_eq!(aligned.row_stride_bytes(), 64);
    assert_eq!(tight.row_stride_bytes(), std::mem::size_of::<Position>());
    assert_eq!(aligned.column_bytes(), 64 * COUNT as usize);
    assert_eq!(aligned.payload_bytes(), COUNT as usize * 16);
    assert_eq!(
        aligned.iter_ordered().collect::<Vec<_>>(),
        tight.iter_ordered().collect::<Vec<_>>()
    );
    // The registry only holds cache-line-aligned columns, so a tight lookup
    // must fail to resolve rather than reinterpret memory.
    assert!(registry.store_in::<Position, TightLayout>().is_none());
}

#[test]
fn query_errors_are_typed_and_planning_never_mutates() {
    let mut registry = ComponentRegistry::new();
    assert!(registry.insert(EntityId::new(0, 0), Position { x: 0, y: 0 }));

    let conflict = registry
        .query::<(&Ref<Position>, &Mut<Position>), QueryAll>()
        .expect_err("double exclusive access must be rejected");
    assert!(matches!(conflict, QueryError::ConflictingAccess { .. }));
    assert!(conflict.to_string().contains("more than once"));

    let missing = ComponentRegistry::new()
        .query_all::<Ref<Velocity>>()
        .expect_err("velocity column is unregistered");
    assert!(matches!(missing, QueryError::SourceColumnMissing { .. }));

    // Building a plan never mutates the registry, so the rejected plans leave
    // the data exactly as it was.
    assert_eq!(registry.row_count::<Position>(), Some(1));
    assert_eq!(
        registry.component_types(EntityId::new(0, 0)),
        vec![registry.register::<Position>()]
    );
}

#[test]
fn mutable_query_is_deterministic_and_visible() {
    let mut world = World::new();
    let entities = spawn_all(&mut world);
    let mut registry = build(&seeds(&entities), false);

    let plan = registry
        .query::<(&Ref<Position>, &Mut<Velocity>), (With<Health>, Without<Frozen>)>()
        .expect("plan builds");

    let mut visited = Vec::new();
    registry.for_each_mut2(
        &plan,
        |entity, position: &Position, velocity: &mut Velocity| {
            visited.push(entity);
            velocity.dx += position.x;
        },
    );

    assert_eq!(visited, plan.entities().to_vec());
    for &entity in &visited {
        // `dx` started at `-index` and gained exactly `index`, so it must be 0.
        let velocity = registry.get::<Velocity>(entity).expect("velocity present");
        assert_eq!(velocity.dx, 0);
    }
}

#[test]
fn filters_are_type_level_and_need_no_value() {
    let mut registry = ComponentRegistry::new();
    assert!(registry.insert(EntityId::new(0, 0), Position { x: 0, y: 0 }));
    assert!(registry.insert(EntityId::new(1, 0), Health(1)));
    assert!(registry.insert(EntityId::new(1, 0), Frozen));
    assert!(registry.insert(EntityId::new(2, 0), Health(2)));

    assert_eq!(
        registry
            .query::<Ref<Position>, With<Health>>()
            .expect("plan builds")
            .len(),
        0
    );
    assert_eq!(
        registry
            .query::<Ref<Position>, Without<Health>>()
            .expect("plan builds")
            .entities(),
        &[EntityId::new(0, 0)]
    );
    assert_eq!(
        registry
            .query::<Ref<Health>, (With<Position>, Without<Frozen>)>()
            .expect("plan builds")
            .entities(),
        &[]
    );
    assert_eq!(
        registry
            .query::<Ref<Health>, (With<Position>, With<Health>)>()
            .expect("plan builds")
            .entities(),
        &[]
    );
    assert_eq!(
        registry
            .query::<Ref<Health>, (Without<Frozen>, With<Health>)>()
            .expect("plan builds")
            .entities(),
        &[EntityId::new(2, 0)]
    );
}
