#![forbid(unsafe_code)]

use kairo_ecs_abm::{DynamicSpawnRequest, DynamicSpawner, SpawnBuffer};
use kairo_ecs_core::{LifecyclePolicy, LinkedEntityError, ModelEntityId, RelationshipKind};
use kairo_ecs_state::{EntityHierarchy, World};
use kairo_ecs_types::{EntityId, SimTime};

#[test]
fn test_dynamic_spawner_budget_and_stale_handles() {
    let mut world = World::new();
    let mut hierarchy = EntityHierarchy::new();
    let base_seed = 42;
    let budget = 3;
    let mut spawner = DynamicSpawner::new(base_seed, budget);

    let parent = world.spawn();
    let parent_model = ModelEntityId::new(500);
    hierarchy
        .register_stable_id(parent_model, parent, &world)
        .unwrap();

    // Spawn 1
    let (rel1, stream1) = spawner
        .spawn_child(
            &mut world,
            &mut hierarchy,
            DynamicSpawnRequest::new(
                parent,
                RelationshipKind::SpawnedFrom,
                LifecyclePolicy::Survive,
                Some(ModelEntityId::new(501)),
                SimTime::from_ticks(10),
            ),
        )
        .unwrap();

    assert_eq!(rel1.parent, parent);
    assert_eq!(rel1.parent_model_id, Some(parent_model));
    assert_eq!(rel1.child_model_id, Some(ModelEntityId::new(501)));
    assert_eq!(rel1.policy, LifecyclePolicy::Survive);
    assert!(world.is_alive(rel1.child));
    assert_eq!(spawner.spawned_count(), 1);
    assert_eq!(spawner.remaining_capacity(), 2);

    // Spawn 2
    let (rel2, _stream2) = spawner
        .spawn_child(
            &mut world,
            &mut hierarchy,
            DynamicSpawnRequest::new(
                parent,
                RelationshipKind::SubtaskOf,
                LifecyclePolicy::CascadeDelete,
                Some(ModelEntityId::new(502)),
                SimTime::from_ticks(20),
            ),
        )
        .unwrap();

    // Spawn 3
    let (rel3, _stream3) = spawner
        .spawn_child(
            &mut world,
            &mut hierarchy,
            DynamicSpawnRequest::new(
                parent,
                RelationshipKind::CargoOf,
                LifecyclePolicy::Detach,
                Some(ModelEntityId::new(503)),
                SimTime::from_ticks(30),
            ),
        )
        .unwrap();

    assert_eq!(spawner.spawned_count(), 3);
    assert_eq!(spawner.remaining_capacity(), 0);

    // Spawn 4: budget exceeded!
    let err = spawner
        .spawn_child(
            &mut world,
            &mut hierarchy,
            DynamicSpawnRequest::new(
                parent,
                RelationshipKind::SpawnedFrom,
                LifecyclePolicy::Survive,
                None,
                SimTime::from_ticks(40),
            ),
        )
        .unwrap_err();
    assert_eq!(
        err,
        LinkedEntityError::CapacityExceeded {
            max_capacity: 3,
            requested: 4
        }
    );

    // Despawn parent with cascade
    hierarchy
        .cascade_despawn(parent, &mut world, SimTime::from_ticks(50))
        .unwrap();

    // Rejection of stale parent handle
    let mut stale_spawner = DynamicSpawner::new(100, 10);
    let err_stale = stale_spawner
        .spawn_child(
            &mut world,
            &mut hierarchy,
            DynamicSpawnRequest::new(
                parent,
                RelationshipKind::SpawnedFrom,
                LifecyclePolicy::Survive,
                None,
                SimTime::from_ticks(50),
            ),
        )
        .unwrap_err();
    assert_eq!(err_stale, LinkedEntityError::StaleEntityHandle(parent));

    // Ensure children 1, 2, 3:
    // rel1 had Survive: still alive, detached from parent
    assert!(world.is_alive(rel1.child));
    assert_eq!(hierarchy.parent_of(rel1.child), None);

    // rel2 had CascadeDelete: despawned
    assert!(!world.is_alive(rel2.child));

    // rel3 had Detach: still alive, detached
    assert!(world.is_alive(rel3.child));
    assert_eq!(hierarchy.parent_of(rel3.child), None);

    // Verify stream1 can generate deterministic numbers
    let mut s = stream1;
    let val = s.next_u64();
    assert_ne!(val, 0);
}

#[test]
fn test_spawn_buffer_mid_iteration_safety() {
    let mut world = World::new();
    let mut hierarchy = EntityHierarchy::new();
    let mut spawner = DynamicSpawner::new(999, 100);
    let mut buffer = SpawnBuffer::new(5);

    let parent = world.spawn();

    // Queue 3 spawns during "inner iteration"
    for i in 0..3 {
        buffer
            .queue_spawn(DynamicSpawnRequest {
                parent,
                kind: RelationshipKind::SubtaskOf,
                policy: LifecyclePolicy::CascadeDelete,
                child_model_id: Some(ModelEntityId::new(2000 + i)),
                at: SimTime::from_ticks((i * 10) as u128),
            })
            .unwrap();
    }

    assert_eq!(buffer.len(), 3);
    assert_eq!(buffer.capacity(), 5);

    // Flush buffer
    let results = spawner
        .flush_buffer(&mut buffer, &mut world, &mut hierarchy)
        .unwrap();

    assert_eq!(results.len(), 3);
    assert!(buffer.is_empty());
    assert_eq!(hierarchy.children_of(parent).len(), 3);

    // Buffer capacity exceeded test
    for i in 0..5 {
        buffer
            .queue_spawn(DynamicSpawnRequest {
                parent,
                kind: RelationshipKind::CargoOf,
                policy: LifecyclePolicy::Survive,
                child_model_id: None,
                at: SimTime::from_ticks(i as u128),
            })
            .unwrap();
    }
    let err = buffer.queue_spawn(DynamicSpawnRequest {
        parent,
        kind: RelationshipKind::CargoOf,
        policy: LifecyclePolicy::Survive,
        child_model_id: None,
        at: SimTime::from_ticks(100),
    });
    assert_eq!(
        err,
        Err(LinkedEntityError::CapacityExceeded {
            max_capacity: 5,
            requested: 6
        })
    );
}

#[test]
fn test_deterministic_spawning_replay() {
    fn run_simulation_run(seed: u64) -> (Vec<EntityId>, Vec<u64>) {
        let mut world = World::new();
        let mut hierarchy = EntityHierarchy::new();
        let mut spawner = DynamicSpawner::new(seed, 10);

        let parent = world.spawn();
        let mut child_ids = Vec::new();
        let mut child_randoms = Vec::new();

        for i in 0..5 {
            let (rel, mut stream) = spawner
                .spawn_child(
                    &mut world,
                    &mut hierarchy,
                    DynamicSpawnRequest::new(
                        parent,
                        RelationshipKind::SpawnedFrom,
                        LifecyclePolicy::Survive,
                        Some(ModelEntityId::new(100 + i)),
                        SimTime::from_ticks((i * 5) as u128),
                    ),
                )
                .unwrap();
            child_ids.push(rel.child);
            child_randoms.push(stream.next_u64());
        }

        (child_ids, child_randoms)
    }

    let run1 = run_simulation_run(12345);
    let run2 = run_simulation_run(12345);
    let run3 = run_simulation_run(99999);

    assert_eq!(run1.0, run2.0, "Entity IDs must match on identical runs");
    assert_eq!(
        run1.1, run2.1,
        "Random streams must match deterministically"
    );

    // Different seed produces different random stream
    assert_ne!(run1.1, run3.1);
}
