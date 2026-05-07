use kairo_ecs_state::{ComponentRegistry, ComponentStore, EntitySnapshot, World};
use kairo_ecs_types::EntityId;

fn entity(index: u64, generation: u32) -> EntityId {
    EntityId { index, generation }
}

#[test]
fn world_entity_lifecycle_recycles_indices_and_rejects_stale_handles() {
    let mut world = World::new();

    let a = world.spawn();
    let b = world.spawn();
    let c = world.spawn();

    assert_eq!(a, entity(0, 0));
    assert_eq!(b, entity(1, 0));
    assert_eq!(c, entity(2, 0));
    assert_eq!(world.len(), 3);
    assert!(world.is_alive(a));
    assert!(world.is_alive(b));
    assert!(world.is_alive(c));

    assert!(world.despawn(b));
    assert!(!world.is_alive(b));
    assert!(!world.despawn(b));
    assert_eq!(world.len(), 2);

    let d = world.spawn();
    assert_eq!(d.index, b.index);
    assert_eq!(d.generation, b.generation.wrapping_add(1));
    assert!(world.is_alive(d));
    assert!(!world.is_alive(b));

    assert_eq!(
        world.snapshot().entities(),
        &[
            EntitySnapshot { id: a },
            EntitySnapshot { id: d },
            EntitySnapshot { id: c },
        ]
    );
}

#[test]
fn world_snapshot_is_deterministic_across_repeated_reads() {
    let mut world = World::new();
    let a = world.spawn();
    let b = world.spawn();
    let c = world.spawn();

    assert!(world.despawn(b));
    let d = world.spawn();

    let snapshot1 = world.snapshot();
    let snapshot2 = world.snapshot();

    assert_eq!(snapshot1, snapshot2);
    assert_eq!(
        snapshot1.entities(),
        &[
            EntitySnapshot { id: a },
            EntitySnapshot { id: d },
            EntitySnapshot { id: c }
        ]
    );
}

#[test]
fn component_store_replaces_same_entity_without_duplicate_row() {
    let mut store = ComponentStore::<u32>::new();
    let entity = entity(10, 0);

    assert!(store.insert(entity, 1));
    assert!(store.insert(entity, 2));

    assert_eq!(store.len(), 1);
    assert_eq!(store.get(entity), Some(&2));
    assert_eq!(store.iter().collect::<Vec<_>>(), vec![(entity, &2)]);
}

#[test]
fn component_store_rejects_stale_generation_for_same_index() {
    let mut store = ComponentStore::<u32>::new();
    let stale = entity(7, 0);
    let current = entity(7, 1);

    assert!(store.insert(current, 42));
    assert!(!store.insert(stale, 99));

    assert_eq!(store.get(stale), None);
    assert_eq!(store.get(current), Some(&42));
    assert_eq!(store.remove(stale), None);
    assert_eq!(store.remove(current), Some(42));
    assert!(store.is_empty());
}

#[test]
fn component_store_newer_generation_supersedes_stale_row() {
    let mut store = ComponentStore::<u32>::new();
    let stale = entity(2, 0);
    let current = entity(2, 1);

    assert!(store.insert(stale, 10));
    assert!(store.insert(current, 20));

    assert_eq!(store.len(), 1);
    assert_eq!(store.get(stale), None);
    assert_eq!(store.get(current), Some(&20));
    assert_eq!(store.iter().collect::<Vec<_>>(), vec![(current, &20)]);
}

#[test]
fn component_registry_multi_type_and_remove() {
    let mut reg = ComponentRegistry::new();
    let e = entity(0, 0);

    assert!(reg.insert(e, 42i32));
    assert!(reg.insert(e, "hello".to_string()));

    assert_eq!(reg.get::<i32>(e), Some(&42));
    assert_eq!(reg.get::<String>(e), Some(&"hello".to_string()));
    assert_eq!(reg.get::<f64>(e), None);

    assert_eq!(reg.remove::<i32>(e), Some(42));
    assert_eq!(reg.get::<i32>(e), None);
}
