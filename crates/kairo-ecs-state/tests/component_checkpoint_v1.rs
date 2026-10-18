use std::cell::Cell;

use kairo_ecs_state::component_checkpoint::{
    ComponentCheckpointError, ComponentCheckpointLimits, ComponentCheckpointStructureError,
    ComponentStoreCheckpointV1,
};
use kairo_ecs_state::{ComponentRegistry, ComponentStore};
use kairo_ecs_types::EntityId;

fn entity(index: u64, generation: u32) -> EntityId {
    EntityId { index, generation }
}

#[derive(Debug, Eq, PartialEq)]
struct NonClone(u64);

fn limits(rows: usize, slots: usize) -> ComponentCheckpointLimits {
    ComponentCheckpointLimits {
        max_rows: rows,
        max_sparse_slots: slots,
    }
}

#[test]
fn borrowed_codec_roundtrips_nonclone_rows_order_holes_and_generations() {
    let mut source = ComponentStore::new();
    let a = entity(1, 3);
    let b = entity(5, 7);
    let c = entity(3, 9);
    assert!(source.insert(a, NonClone(10)));
    assert!(source.insert(b, NonClone(20)));
    assert!(source.insert(c, NonClone(30)));
    assert_eq!(source.remove(b), Some(NonClone(20)));
    assert!(source.insert(entity(7, 2), NonClone(40)));

    let image = source
        .checkpoint_state_with(limits(8, 16), |value| Ok::<_, ()>(value.0))
        .unwrap();
    assert_eq!(image.sparse_slots, 8);
    assert_eq!(
        image.rows.iter().map(|row| row.0).collect::<Vec<_>>(),
        [a, c, entity(7, 2)]
    );
    let restored = ComponentStore::from_checkpoint_state_with(image, limits(8, 16), |value| {
        Ok::<_, ()>(NonClone(value))
    })
    .unwrap();
    assert_eq!(restored.len(), 3);
    assert_eq!(
        restored.iter().map(|(id, v)| (id, v.0)).collect::<Vec<_>>(),
        [(a, 10), (c, 30), (entity(7, 2), 40)]
    );
    assert_eq!(restored.get(a), Some(&NonClone(10)));
    assert_eq!(restored.get(c), Some(&NonClone(30)));
    assert_eq!(restored.get(entity(7, 2)), Some(&NonClone(40)));
    assert_eq!(restored.get(entity(1, 2)), None);

    let mut restored = restored;
    let stale = entity(1, 2);
    let newer = entity(1, 4);
    assert!(!source.insert(stale, NonClone(99)));
    assert!(!restored.insert(stale, NonClone(99)));
    assert!(source.insert(newer, NonClone(11)));
    assert!(restored.insert(newer, NonClone(11)));
    assert_eq!(restored.get(a), None);
    assert_eq!(restored.get(stale), None);
    assert_eq!(restored.get(newer), Some(&NonClone(11)));

    assert_eq!(source.remove(c), Some(NonClone(30)));
    assert_eq!(restored.remove(c), Some(NonClone(30)));
    let next = entity(9, 6);
    assert!(source.insert(next, NonClone(50)));
    assert!(restored.insert(next, NonClone(50)));
    assert_eq!(
        source.iter().collect::<Vec<_>>(),
        restored.iter().collect::<Vec<_>>()
    );
    let source_ids = source
        .iter()
        .map(|(id, value)| (id, value.0))
        .collect::<Vec<_>>();
    let restored_ids = restored
        .iter()
        .map(|(id, value)| (id, value.0))
        .collect::<Vec<_>>();
    assert_eq!(source_ids, restored_ids);
    assert_eq!(source.get(next), restored.get(next));
    let source_image = source
        .checkpoint_state_with(limits(8, 16), |value| Ok::<_, ()>(value.0))
        .unwrap();
    let restored_image = restored
        .checkpoint_state_with(limits(8, 16), |value| Ok::<_, ()>(value.0))
        .unwrap();
    assert_eq!(source_image, restored_image);
    assert_eq!(restored_image.sparse_slots, 10);
}

#[test]
fn empty_store_preserves_nonzero_sparse_length_and_accepts_future_rows() {
    let mut source = ComponentStore::new();
    let temporary = entity(12, 5);
    assert!(source.insert(temporary, NonClone(1)));
    assert_eq!(source.remove(temporary), Some(NonClone(1)));

    let image = source
        .checkpoint_state_with(limits(4, 16), |value| Ok::<_, ()>(value.0))
        .unwrap();
    assert!(image.rows.is_empty());
    assert_eq!(image.sparse_slots, 13);
    let mut restored = ComponentStore::from_checkpoint_state_with(image, limits(4, 16), |value| {
        Ok::<_, ()>(NonClone(value))
    })
    .unwrap();
    assert!(restored.is_empty());
    assert_eq!(
        restored
            .checkpoint_state_with(limits(4, 16), |value| Ok::<_, ()>(value.0))
            .unwrap()
            .sparse_slots,
        13
    );
    let future = entity(4, 8);
    assert!(restored.insert(future, NonClone(2)));
    assert_eq!(restored.get(future), Some(&NonClone(2)));
}

#[test]
fn invalid_images_are_rejected_before_decoder_runs() {
    let invalid = [
        (
            ComponentStoreCheckpointV1 {
                version: 2,
                sparse_slots: 1,
                rows: vec![],
            },
            limits(2, 2),
            ComponentCheckpointStructureError::UnsupportedVersion(2),
        ),
        (
            ComponentStoreCheckpointV1 {
                version: 1,
                sparse_slots: 1,
                rows: vec![(entity(0, 0), 1), (entity(1, 0), 2)],
            },
            limits(1, 2),
            ComponentCheckpointStructureError::RowLimitExceeded,
        ),
        (
            ComponentStoreCheckpointV1 {
                version: 1,
                sparse_slots: 3,
                rows: vec![],
            },
            limits(2, 2),
            ComponentCheckpointStructureError::SparseSlotLimitExceeded,
        ),
        (
            ComponentStoreCheckpointV1 {
                version: 1,
                sparse_slots: 1,
                rows: vec![(entity(0, 0), 1), (entity(0, 1), 2)],
            },
            limits(2, 2),
            ComponentCheckpointStructureError::DuplicateEntity,
        ),
    ];
    for (image, caps, expected) in invalid {
        let called = Cell::new(false);
        let result = ComponentStore::<u32>::from_checkpoint_state_with(image, caps, |v| {
            called.set(true);
            Ok::<_, ()>(v)
        });
        assert!(
            matches!(result, Err(ComponentCheckpointError::Structure(error)) if error == expected)
        );
        assert!(!called.get());
    }
}

#[test]
fn invalid_index_is_rejected_before_decoder_runs() {
    let called = Cell::new(false);
    let result = ComponentStore::<u32>::from_checkpoint_state_with(
        ComponentStoreCheckpointV1 {
            version: 1,
            sparse_slots: 2,
            rows: vec![(entity(2, 0), 5)],
        },
        limits(2, 2),
        |v| {
            called.set(true);
            Ok::<_, ()>(v)
        },
    );
    assert!(matches!(
        result,
        Err(ComponentCheckpointError::Structure(_))
    ));
    assert!(!called.get());
}

#[test]
fn codec_failure_does_not_return_a_partial_store() {
    let image = ComponentStoreCheckpointV1 {
        version: 1,
        sparse_slots: 3,
        rows: vec![(entity(0, 0), 1), (entity(2, 4), 2)],
    };
    let mut calls = 0;
    let result = ComponentStore::<u32>::from_checkpoint_state_with(image, limits(2, 3), |v| {
        calls += 1;
        if v == 2 {
            Err("bad payload")
        } else {
            Ok(v)
        }
    });
    assert_eq!(calls, 2);
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("codec error unexpectedly returned a store"),
    };
    assert!(matches!(
        error,
        ComponentCheckpointError::Codec("bad payload")
    ));
}

#[test]
fn registry_type_inventory_is_native_and_includes_empty_stores() {
    let mut registry = ComponentRegistry::new();
    registry.register::<u32>();
    registry.register::<String>();
    let types = registry.registered_types();
    assert_eq!(types.len(), 2);
    assert!(types.contains(&std::any::TypeId::of::<u32>()));
    assert!(types.contains(&std::any::TypeId::of::<String>()));
}

#[test]
fn large_sparse_length_does_not_change_linear_membership_contract() {
    let rows = (0..20_000)
        .map(|i| (entity(i * 2, (i % 13) as u32), i))
        .collect();
    let image = ComponentStoreCheckpointV1 {
        version: 1,
        sparse_slots: 40_000,
        rows,
    };
    let restored =
        ComponentStore::from_checkpoint_state_with(image, limits(20_000, 40_000), Ok::<_, ()>)
            .unwrap();
    assert_eq!(restored.len(), 20_000);
}
