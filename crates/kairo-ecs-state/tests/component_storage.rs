use kairo_ecs_state::{ComponentRegistry, ComponentStore, World};
use kairo_ecs_types::EntityId;

fn entity(idx: u64) -> EntityId {
    EntityId {
        index: idx,
        generation: 0,
    }
}

#[test]
fn test_sparse_set_insert_get() {
    let mut store = ComponentStore::<i32>::new();
    let e0 = entity(0);
    let e1 = entity(1);
    let e2 = entity(2);

    store.insert(e0, 10);
    store.insert(e1, 20);
    store.insert(e2, 30);

    assert_eq!(store.get(e0), Some(&10));
    assert_eq!(store.get(e1), Some(&20));
    assert_eq!(store.get(e2), Some(&30));
    assert_eq!(store.get(entity(99)), None);
}

#[test]
fn test_sparse_set_remove() {
    let mut store = ComponentStore::<i32>::new();
    let e0 = entity(0);
    let e1 = entity(1);
    let e2 = entity(2);

    store.insert(e0, 100);
    store.insert(e1, 200);
    store.insert(e2, 300);
    assert_eq!(store.len(), 3);

    let removed = store.remove(e1);
    assert_eq!(removed, Some(200));
    assert_eq!(store.len(), 2);
    assert_eq!(store.get(e1), None);

    assert_eq!(store.get(e0), Some(&100));
    assert_eq!(store.get(e2), Some(&300));
}

#[test]
fn test_sparse_set_iter() {
    let mut store = ComponentStore::<i32>::new();
    let ids: Vec<EntityId> = (0..5).map(entity).collect();
    for (i, &id) in ids.iter().enumerate() {
        store.insert(id, (i as i32) * 10);
    }

    let mut collected: Vec<(EntityId, &i32)> = store.iter().collect();
    collected.sort_by_key(|(id, _)| id.index);

    assert_eq!(collected.len(), 5);
    for (i, (id, val)) in collected.iter().enumerate() {
        assert_eq!(id.index, i as u64);
        assert_eq!(**val, (i as i32) * 10);
    }
}

#[test]
fn test_sparse_set_iter_mut() {
    let mut store = ComponentStore::<i32>::new();
    let ids: Vec<EntityId> = (0..3).map(entity).collect();
    for (i, &id) in ids.iter().enumerate() {
        store.insert(id, (i as i32) * 5);
    }

    for (_, val) in store.iter_mut() {
        *val += 1;
    }

    assert_eq!(store.get(ids[0]), Some(&1));
    assert_eq!(store.get(ids[1]), Some(&6));
    assert_eq!(store.get(ids[2]), Some(&11));
}

#[test]
fn test_sparse_set_contains() {
    let mut store = ComponentStore::<i32>::new();
    let e = entity(5);
    assert!(!store.contains(e));

    store.insert(e, 42);
    assert!(store.contains(e));

    store.remove(e);
    assert!(!store.contains(e));
}

#[test]
fn test_sparse_set_empty() {
    let store = ComponentStore::<i32>::new();
    assert!(store.is_empty());
    assert_eq!(store.len(), 0);
}

#[test]
fn test_sparse_set_swap_remove_correctness() {
    let mut store = ComponentStore::<i32>::new();
    let ids: Vec<EntityId> = (0..10).map(entity).collect();
    for (i, &id) in ids.iter().enumerate() {
        store.insert(id, i as i32);
    }
    assert_eq!(store.len(), 10);

    // Remove entities 1, 3, 5, 7, 9 (odd indices)
    for &id in &[ids[1], ids[3], ids[5], ids[7], ids[9]] {
        store.remove(id);
    }
    assert_eq!(store.len(), 5);

    // Verify remaining entries match original values
    for &id in &[ids[0], ids[2], ids[4], ids[6], ids[8]] {
        let val = store.get(id).copied().unwrap();
        assert_eq!(val, id.index as i32);
    }

    // Verify removed entries are gone
    for &id in &[ids[1], ids[3], ids[5], ids[7], ids[9]] {
        assert_eq!(store.get(id), None);
    }
}

#[test]
fn test_component_registry_multi_type() {
    let mut reg = ComponentRegistry::new();
    let e = entity(0);

    reg.insert(e, 42i32);
    reg.insert(e, "hello".to_string());

    assert_eq!(reg.get::<i32>(e), Some(&42));
    assert_eq!(reg.get::<String>(e), Some(&"hello".to_string()));
    assert_eq!(reg.get::<f64>(e), None);
}

#[test]
fn test_component_registry_remove_type() {
    let mut reg = ComponentRegistry::new();
    let e = entity(0);

    reg.insert(e, 99i32);
    assert_eq!(reg.get::<i32>(e), Some(&99));

    let removed = reg.remove::<i32>(e);
    assert_eq!(removed, Some(99));
    assert_eq!(reg.get::<i32>(e), None);
}

#[test]
fn test_world_entity_lifecycle() {
    let mut world = World::new();
    assert!(world.is_empty());
    assert_eq!(world.len(), 0);

    let a = world.spawn();
    let b = world.spawn();
    let c = world.spawn();
    assert_eq!(world.len(), 3);
    assert!(world.is_alive(a));
    assert!(world.is_alive(b));
    assert!(world.is_alive(c));

    assert!(world.despawn(b));
    assert_eq!(world.len(), 2);
    assert!(!world.is_alive(b));
    assert!(world.is_alive(a));
    assert!(world.is_alive(c));

    assert!(!world.despawn(b)); // already despawned
    assert_eq!(world.len(), 2);

    world.despawn(a);
    world.despawn(c);
    assert!(world.is_empty());
}

#[test]
fn test_world_snapshot() {
    let mut world = World::new();
    let _a = world.spawn();
    let b = world.spawn();

    let snap1 = world.snapshot();
    assert_eq!(snap1.len(), 2);
    assert_eq!(snap1.is_empty(), false);

    let _c = world.spawn();
    let _d = world.spawn();

    // snapshot should be unchanged
    assert_eq!(snap1.len(), 2);
    assert_eq!(snap1.entities().len(), 2);

    let snap2 = world.snapshot();
    assert_eq!(snap2.len(), 4);

    world.despawn(b);
    let snap3 = world.snapshot();
    assert_eq!(snap3.len(), 3);

    let ids: Vec<u64> = snap3.entities().iter().map(|e| e.id.index).collect();
    assert_eq!(ids, vec![0, 2, 3]);
}
