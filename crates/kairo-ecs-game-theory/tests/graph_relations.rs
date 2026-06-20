#![cfg(feature = "graph-relations")]

use kairo_ecs_game_theory::graph_relations::{ChildOf, TransitionTo};
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
    assert_eq!(transitions.get(current).map(TransitionTo::target), Some(next));
    assert_eq!(child_of.iter().map(|(id, edge)| (id, edge.target())).collect::<Vec<_>>(), vec![(child, parent)]);
}
