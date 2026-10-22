#![forbid(unsafe_code)]

use kairo_ecs_core::{
    LifecyclePolicy, LinkedEntityError, ModelEntityId, RelationshipKind, RelationshipLifecycleEvent,
};
use kairo_ecs_state::{EntityHierarchy, World};
use kairo_ecs_types::{EntityId, SimTime};

#[test]
fn test_hierarchy_one_to_many_cardinality() {
    let mut world = World::new();
    let mut hierarchy = EntityHierarchy::new();

    let parent = world.spawn();
    let child1 = world.spawn();
    let child2 = world.spawn();
    let child3 = world.spawn();

    assert!(hierarchy
        .attach_child(
            parent,
            child1,
            RelationshipKind::ParentChild,
            LifecyclePolicy::Survive,
            &world,
            SimTime::from_ticks(10),
        )
        .is_ok());

    assert!(hierarchy
        .attach_child(
            parent,
            child2,
            RelationshipKind::SubtaskOf,
            LifecyclePolicy::CascadeDelete,
            &world,
            SimTime::from_ticks(12),
        )
        .is_ok());

    assert!(hierarchy
        .attach_child(
            parent,
            child3,
            RelationshipKind::CargoOf,
            LifecyclePolicy::Detach,
            &world,
            SimTime::from_ticks(15),
        )
        .is_ok());

    assert_eq!(hierarchy.children_of(parent), vec![child1, child2, child3]);
    assert_eq!(hierarchy.parent_of(child1), Some(parent));
    assert_eq!(hierarchy.parent_of(child2), Some(parent));
    assert_eq!(hierarchy.parent_of(child3), Some(parent));

    assert_eq!(
        hierarchy.typed_children_of(parent, RelationshipKind::SubtaskOf),
        vec![child2]
    );
    assert_eq!(
        hierarchy.typed_children_of(parent, RelationshipKind::CargoOf),
        vec![child3]
    );
}

#[test]
fn test_hierarchy_cycle_rejection() {
    let mut world = World::new();
    let mut hierarchy = EntityHierarchy::new();

    let a = world.spawn();
    let b = world.spawn();
    let c = world.spawn();

    // Self-parenting cycle rejection
    let err = hierarchy.attach_child(
        a,
        a,
        RelationshipKind::ParentChild,
        LifecyclePolicy::Survive,
        &world,
        SimTime::ZERO,
    );
    assert_eq!(
        err,
        Err(LinkedEntityError::CycleDetected {
            parent: a,
            child: a
        })
    );

    // a -> b -> c
    assert!(hierarchy
        .attach_child(
            a,
            b,
            RelationshipKind::ParentChild,
            LifecyclePolicy::Survive,
            &world,
            SimTime::ZERO,
        )
        .is_ok());

    assert!(hierarchy
        .attach_child(
            b,
            c,
            RelationshipKind::ParentChild,
            LifecyclePolicy::Survive,
            &world,
            SimTime::ZERO,
        )
        .is_ok());

    // Try c -> a (cycle!)
    let err = hierarchy.attach_child(
        c,
        a,
        RelationshipKind::ParentChild,
        LifecyclePolicy::Survive,
        &world,
        SimTime::ZERO,
    );
    assert_eq!(
        err,
        Err(LinkedEntityError::CycleDetected {
            parent: c,
            child: a
        })
    );

    // Reparenting cycle rejection: reparent a under c
    let err = hierarchy.reparent(a, c, &world, SimTime::ZERO);
    assert_eq!(
        err,
        Err(LinkedEntityError::CycleDetected {
            parent: c,
            child: a
        })
    );
}

#[test]
fn test_hierarchy_stale_handle_rejection() {
    let mut world = World::new();
    let mut hierarchy = EntityHierarchy::new();

    let parent = world.spawn();
    let child = world.spawn();

    world.despawn(parent);

    let err = hierarchy.attach_child(
        parent,
        child,
        RelationshipKind::ParentChild,
        LifecyclePolicy::Survive,
        &world,
        SimTime::ZERO,
    );
    assert_eq!(err, Err(LinkedEntityError::StaleEntityHandle(parent)));

    let stale_child = EntityId::new(999, 1);
    let err = hierarchy.attach_child(
        child,
        stale_child,
        RelationshipKind::ParentChild,
        LifecyclePolicy::Survive,
        &world,
        SimTime::ZERO,
    );
    assert_eq!(err, Err(LinkedEntityError::StaleEntityHandle(stale_child)));
}

#[test]
fn test_hierarchy_duplicate_relationship_rejection() {
    let mut world = World::new();
    let mut hierarchy = EntityHierarchy::new();

    let parent = world.spawn();
    let child = world.spawn();

    assert!(hierarchy
        .attach_child(
            parent,
            child,
            RelationshipKind::ParentChild,
            LifecyclePolicy::Survive,
            &world,
            SimTime::ZERO,
        )
        .is_ok());

    // Duplicate attachment with same parent and kind
    let err = hierarchy.attach_child(
        parent,
        child,
        RelationshipKind::ParentChild,
        LifecyclePolicy::Survive,
        &world,
        SimTime::ZERO,
    );
    assert_eq!(
        err,
        Err(LinkedEntityError::DuplicateRelationship { parent, child })
    );
}

#[test]
fn test_hierarchy_stable_model_id_mapping() {
    let mut world = World::new();
    let mut hierarchy = EntityHierarchy::new();

    let entity1 = world.spawn();
    let entity2 = world.spawn();

    let model1 = ModelEntityId::new(1001);
    let model2 = ModelEntityId::new(1002);

    assert!(hierarchy
        .register_stable_id(model1, entity1, &world)
        .is_ok());
    assert!(hierarchy
        .register_stable_id(model2, entity2, &world)
        .is_ok());

    // Lookup
    assert_eq!(hierarchy.resolve_stable_id(model1, &world), Some(entity1));
    assert_eq!(hierarchy.resolve_stable_id(model2, &world), Some(entity2));
    assert_eq!(hierarchy.get_stable_id(entity1), Some(model1));

    // Collision detection: cannot assign model1 to entity2
    let err = hierarchy.register_stable_id(model1, entity2, &world);
    assert_eq!(err, Err(LinkedEntityError::StableIdCollision(model1)));

    // Despawn entity1 and verify resolve returns None
    world.despawn(entity1);
    assert_eq!(hierarchy.resolve_stable_id(model1, &world), None);
}

#[test]
fn test_hierarchy_explicit_detach_and_reparent() {
    let mut world = World::new();
    let mut hierarchy = EntityHierarchy::new();

    let p1 = world.spawn();
    let p2 = world.spawn();
    let c = world.spawn();

    hierarchy
        .attach_child(
            p1,
            c,
            RelationshipKind::SubtaskOf,
            LifecyclePolicy::CascadeDelete,
            &world,
            SimTime::from_ticks(1),
        )
        .unwrap();

    assert_eq!(hierarchy.parent_of(c), Some(p1));
    assert_eq!(hierarchy.children_of(p1), vec![c]);

    // Reparent c under p2
    hierarchy
        .reparent(c, p2, &world, SimTime::from_ticks(2))
        .unwrap();
    assert_eq!(hierarchy.parent_of(c), Some(p2));
    assert_eq!(hierarchy.children_of(p1), Vec::<EntityId>::new());
    assert_eq!(hierarchy.children_of(p2), vec![c]);

    // Detach c from p2
    hierarchy
        .detach_child(p2, c, SimTime::from_ticks(3))
        .unwrap();
    assert_eq!(hierarchy.parent_of(c), None);
    assert_eq!(hierarchy.children_of(p2), Vec::<EntityId>::new());
}

#[test]
fn test_hierarchy_cascade_and_survive_lifecycle_parity() {
    let mut world = World::new();
    let mut hierarchy = EntityHierarchy::new();

    let root = world.spawn();
    let cascade_child = world.spawn();
    let cascade_grandchild = world.spawn();
    let surviving_child = world.spawn();
    let surviving_child_subtask = world.spawn();

    // root -> cascade_child (CascadeDelete) -> cascade_grandchild (CascadeDelete)
    hierarchy
        .attach_child(
            root,
            cascade_child,
            RelationshipKind::ParentChild,
            LifecyclePolicy::CascadeDelete,
            &world,
            SimTime::from_ticks(10),
        )
        .unwrap();

    hierarchy
        .attach_child(
            cascade_child,
            cascade_grandchild,
            RelationshipKind::ParentChild,
            LifecyclePolicy::CascadeDelete,
            &world,
            SimTime::from_ticks(11),
        )
        .unwrap();

    // root -> surviving_child (Survive) -> surviving_child_subtask (CascadeDelete under surviving child)
    hierarchy
        .attach_child(
            root,
            surviving_child,
            RelationshipKind::ParentChild,
            LifecyclePolicy::Survive,
            &world,
            SimTime::from_ticks(12),
        )
        .unwrap();

    hierarchy
        .attach_child(
            surviving_child,
            surviving_child_subtask,
            RelationshipKind::SubtaskOf,
            LifecyclePolicy::CascadeDelete,
            &world,
            SimTime::from_ticks(13),
        )
        .unwrap();

    // Despawn root with cascade
    let despawned = hierarchy
        .cascade_despawn(root, &mut world, SimTime::from_ticks(20))
        .unwrap();

    // Root, cascade_child, cascade_grandchild should be despawned
    assert!(despawned.contains(&root));
    assert!(despawned.contains(&cascade_child));
    assert!(despawned.contains(&cascade_grandchild));

    // In World:
    assert!(!world.is_alive(root));
    assert!(!world.is_alive(cascade_child));
    assert!(!world.is_alive(cascade_grandchild));

    // surviving_child and its subtask MUST survive
    assert!(world.is_alive(surviving_child));
    assert!(world.is_alive(surviving_child_subtask));

    // surviving_child is now an independent root (detached from root)
    assert_eq!(hierarchy.parent_of(surviving_child), None);
    // surviving_child still has its subtask
    assert_eq!(
        hierarchy.children_of(surviving_child),
        vec![surviving_child_subtask]
    );

    // Verify events recorded
    let events = hierarchy.drain_events();
    assert!(events.iter().any(|e| matches!(
        e,
        RelationshipLifecycleEvent::CascadedDespawn {
            entity,
            cascade_count,
            ..
        } if *entity == root && *cascade_count == 3
    )));
    assert!(events.iter().any(|e| matches!(
        e,
        RelationshipLifecycleEvent::Detached { parent, child, .. } if *parent == root && *child == surviving_child
    )));
}
