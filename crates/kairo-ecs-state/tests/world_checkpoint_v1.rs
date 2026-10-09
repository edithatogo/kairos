use kairo_ecs_state::checkpoint::{
    WorldCheckpointError, WorldCheckpointLimits, WorldCheckpointV1, WorldSlotCheckpointV1,
};
use kairo_ecs_state::World;
use kairo_ecs_types::EntityId;

fn limits() -> WorldCheckpointLimits {
    WorldCheckpointLimits { max_slots: 32 }
}

fn transported(checkpoint: &WorldCheckpointV1) -> WorldCheckpointV1 {
    WorldCheckpointV1 {
        version: checkpoint.version,
        slots: checkpoint
            .slots
            .iter()
            .map(|slot| WorldSlotCheckpointV1 {
                generation: slot.generation,
                alive: slot.alive,
            })
            .collect(),
        free_indices: checkpoint.free_indices.clone(),
        live_entities: checkpoint.live_entities.clone(),
    }
}

#[test]
fn transport_preserves_lifo_reuse_generations_and_fragmented_state() {
    let mut original = World::new();
    let ids: Vec<_> = (0..6).map(|_| original.spawn()).collect();
    assert!(original.despawn(ids[1]));
    assert!(original.despawn(ids[4]));
    assert!(original.despawn(ids[3]));
    let checkpoint = original.checkpoint_state(limits()).unwrap();
    assert_eq!(checkpoint.free_indices, vec![1, 4, 3]);
    assert_eq!(checkpoint.live_entities, vec![ids[0], ids[5], ids[2]]);

    let imported = World::from_checkpoint_state(transported(&checkpoint), limits()).unwrap();
    assert_eq!(
        imported.checkpoint_state(limits()).unwrap(),
        checkpoint,
        "transported DTO retains allocator stacks and dense order"
    );

    let mut imported = imported;
    for _ in 0..4 {
        let expected = original.spawn();
        assert_eq!(expected, imported.spawn());
    }
    for stale_index in [1, 4, 3] {
        let stale = EntityId::new(stale_index, 0);
        assert!(!original.is_alive(stale));
        assert!(!imported.is_alive(stale));
        assert!(!original.despawn(stale));
        assert!(!imported.despawn(stale));
    }
    for index in [0, 5, 2, 1, 4, 3] {
        let id = EntityId::new(
            index,
            if index == 1 || index == 4 || index == 3 {
                1
            } else {
                0
            },
        );
        assert_eq!(original.despawn(id), imported.despawn(id));
    }
    assert_eq!(original.snapshot(), imported.snapshot());
    assert_eq!(
        original.checkpoint_state(limits()).unwrap(),
        imported.checkpoint_state(limits()).unwrap()
    );
}

#[test]
fn import_rejects_bad_schema_limits_and_allocator_partitions() {
    let mut world = World::new();
    let first = world.spawn();
    let second = world.spawn();
    let third = world.spawn();
    let fourth = world.spawn();
    assert!(world.despawn(first));
    assert!(world.despawn(third));
    let source = world.checkpoint_state(limits()).unwrap();

    let mut bad = transported(&source);
    bad.version = 2;
    assert_eq!(
        World::from_checkpoint_state(bad, limits()).unwrap_err(),
        WorldCheckpointError::UnsupportedVersion(2)
    );

    let mut bad = transported(&source);
    bad.free_indices[0] = 99;
    assert_eq!(
        World::from_checkpoint_state(bad, limits()).unwrap_err(),
        WorldCheckpointError::InvalidState
    );

    let mut bad = transported(&source);
    bad.free_indices[1] = bad.free_indices[0];
    assert_eq!(
        World::from_checkpoint_state(bad, limits()).unwrap_err(),
        WorldCheckpointError::InvalidState
    );

    let mut bad = transported(&source);
    bad.live_entities[0].generation = bad.live_entities[0].generation.wrapping_add(1);
    assert_eq!(
        World::from_checkpoint_state(bad, limits()).unwrap_err(),
        WorldCheckpointError::InvalidState
    );

    let mut bad = transported(&source);
    bad.free_indices[0] = second.index;
    assert_eq!(
        World::from_checkpoint_state(bad, limits()).unwrap_err(),
        WorldCheckpointError::InvalidState
    );

    let mut bad = transported(&source);
    bad.live_entities[1] = bad.live_entities[0];
    assert_eq!(
        World::from_checkpoint_state(bad, limits()).unwrap_err(),
        WorldCheckpointError::InvalidState
    );

    let mut bad = transported(&source);
    bad.live_entities.pop();
    bad.free_indices.pop();
    assert_eq!(
        World::from_checkpoint_state(bad, limits()).unwrap_err(),
        WorldCheckpointError::InvalidState
    );

    assert_eq!(
        World::from_checkpoint_state(transported(&source), WorldCheckpointLimits { max_slots: 1 })
            .unwrap_err(),
        WorldCheckpointError::LimitExceeded
    );
    assert_eq!(world.checkpoint_state(limits()).unwrap(), source);
    assert!(world.is_alive(second));
    assert!(world.is_alive(fourth));
}

#[test]
fn capture_limits_precede_export_and_empty_world_roundtrips() {
    let mut world = World::new();
    let empty_checkpoint = world
        .checkpoint_state(WorldCheckpointLimits { max_slots: 0 })
        .unwrap();
    assert_eq!(
        empty_checkpoint,
        WorldCheckpointV1 {
            version: 1,
            slots: vec![],
            free_indices: vec![],
            live_entities: vec![],
        }
    );
    let empty_restored = World::from_checkpoint_state(
        transported(&empty_checkpoint),
        WorldCheckpointLimits { max_slots: 0 },
    )
    .unwrap();
    assert!(empty_restored.is_empty());
    let id = world.spawn();
    assert_eq!(
        world
            .checkpoint_state(WorldCheckpointLimits { max_slots: 0 })
            .unwrap_err(),
        WorldCheckpointError::LimitExceeded
    );
    assert!(world.is_alive(id));
    let restored =
        World::from_checkpoint_state(world.checkpoint_state(limits()).unwrap(), limits()).unwrap();
    assert_eq!(restored.snapshot(), world.snapshot());
}

#[test]
fn fragmented_hundred_thousand_slot_restore_preserves_future_ids() {
    const SLOT_COUNT: usize = 100_000;
    let mut original = World::new();
    let original_ids: Vec<_> = (0..SLOT_COUNT).map(|_| original.spawn()).collect();
    for index in (0..SLOT_COUNT).step_by(2) {
        assert!(original.despawn(original_ids[index]));
    }

    let limits = WorldCheckpointLimits {
        max_slots: SLOT_COUNT + 1,
    };
    let checkpoint = original.checkpoint_state(limits).unwrap();
    let mut restored = World::from_checkpoint_state(transported(&checkpoint), limits).unwrap();

    for _ in 0..(SLOT_COUNT / 2) {
        assert_eq!(original.spawn(), restored.spawn());
    }
    assert_eq!(original.spawn(), restored.spawn());
    assert_eq!(
        original.checkpoint_state(limits).unwrap(),
        restored.checkpoint_state(limits).unwrap()
    );
}

#[test]
fn maximum_generation_wraps_and_recycles_identically_after_restore() {
    fn maximum_generation_checkpoint() -> WorldCheckpointV1 {
        WorldCheckpointV1 {
            version: 1,
            slots: vec![WorldSlotCheckpointV1 {
                generation: u32::MAX,
                alive: true,
            }],
            free_indices: vec![],
            live_entities: vec![EntityId::new(0, u32::MAX)],
        }
    }

    let mut uninterrupted =
        World::from_checkpoint_state(maximum_generation_checkpoint(), limits()).unwrap();
    let mut restored =
        World::from_checkpoint_state(maximum_generation_checkpoint(), limits()).unwrap();
    let max_generation = EntityId::new(0, u32::MAX);
    assert!(uninterrupted.despawn(max_generation));
    assert!(restored.despawn(max_generation));
    assert!(!uninterrupted.is_alive(max_generation));
    assert!(!restored.is_alive(max_generation));
    let recycled = EntityId::new(0, 0);
    assert_eq!(uninterrupted.spawn(), recycled);
    assert_eq!(restored.spawn(), recycled);
    assert!(uninterrupted.is_alive(recycled));
    assert!(restored.is_alive(recycled));
}
