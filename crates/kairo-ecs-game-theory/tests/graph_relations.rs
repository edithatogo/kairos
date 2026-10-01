#![cfg(feature = "graph-relations")]

use kairo_ecs_game_theory::graph_relations::{
    children_of, depth_first_descendants, transition_target, ChildOf, TransitionTo,
};
use kairo_ecs_state::ComponentStore;
use kairo_ecs_types::EntityId;

fn entity(index: u64) -> EntityId {
    EntityId::new(index, 0)
}

#[test]
fn relationship_components_store_entity_ids_as_copy_data() {
    let parent = entity(1);
    let child_edge = ChildOf(parent);
    let transition_edge = TransitionTo(entity(2));

    let copied_child = child_edge;
    let copied_transition = transition_edge;

    assert_eq!(copied_child.target(), parent);
    assert_eq!(copied_transition.target(), entity(2));
}

#[test]
fn relationship_components_live_in_dense_component_stores() {
    let parent = entity(10);
    let child = entity(11);
    let current = entity(20);
    let next = entity(21);

    let mut child_of = ComponentStore::new();
    let mut transitions = ComponentStore::new();

    assert!(child_of.insert(child, ChildOf(parent)));
    assert!(transitions.insert(current, TransitionTo(next)));

    assert_eq!(child_of.get(child).map(ChildOf::target), Some(parent));
    assert_eq!(
        transitions.get(current).map(TransitionTo::target),
        Some(next)
    );
    assert_eq!(
        child_of
            .iter()
            .map(|(id, edge)| (id, edge.target()))
            .collect::<Vec<_>>(),
        vec![(child, parent)]
    );
}

#[test]
fn children_query_scans_flat_component_store() {
    let parent = entity(30);
    let first_child = entity(31);
    let second_child = entity(32);
    let unrelated = entity(33);

    let mut child_of = ComponentStore::new();
    assert!(child_of.insert(first_child, ChildOf(parent)));
    assert!(child_of.insert(unrelated, ChildOf(entity(99))));
    assert!(child_of.insert(second_child, ChildOf(parent)));

    let children = children_of(parent, &child_of).collect::<Vec<_>>();

    assert_eq!(children, vec![first_child, second_child]);
}

#[test]
fn transition_query_reads_target_from_flat_component_store() {
    let current = entity(40);
    let next = entity(41);
    let missing = entity(42);

    let mut transitions = ComponentStore::new();
    assert!(transitions.insert(current, TransitionTo(next)));

    assert_eq!(transition_target(current, &transitions), Some(next));
    assert_eq!(transition_target(missing, &transitions), None);
}

#[test]
fn descendant_traversal_walks_entity_id_edges_without_recursion() {
    let root = entity(50);
    let left = entity(51);
    let right = entity(52);
    let grandchild = entity(53);

    let mut child_of = ComponentStore::new();
    assert!(child_of.insert(left, ChildOf(root)));
    assert!(child_of.insert(right, ChildOf(root)));
    assert!(child_of.insert(grandchild, ChildOf(left)));

    let descendants = depth_first_descendants(root, &child_of);

    assert_eq!(descendants, vec![left, grandchild, right]);
}

#[test]
fn descendant_traversal_stops_at_seen_entities() {
    let root = entity(60);
    let child = entity(61);
    let cycle = entity(62);

    let mut child_of = ComponentStore::new();
    assert!(child_of.insert(child, ChildOf(root)));
    assert!(child_of.insert(cycle, ChildOf(child)));
    assert!(child_of.insert(root, ChildOf(cycle)));

    let descendants = depth_first_descendants(root, &child_of);

    assert_eq!(descendants, vec![child, cycle]);
}
