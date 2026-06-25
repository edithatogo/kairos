#![cfg(feature = "graph-relations")]

use kairo_ecs_game_theory::extensive_form::{
    information_set_fixtures, ActionEdge, ExtensiveFormError, ExtensiveNode,
    ExtensiveTraversalStores, InformationSet,
};
use kairo_ecs_game_theory::graph_relations::{ChildOf, TransitionTo};
use kairo_ecs_state::ComponentStore;
use kairo_ecs_types::EntityId;

fn entity(index: u64) -> EntityId {
    EntityId::new(index, 0)
}

fn add_action(
    source: EntityId,
    edge: EntityId,
    label: &str,
    target: EntityId,
    child_of: &mut ComponentStore<ChildOf>,
    actions: &mut ComponentStore<ActionEdge>,
    transitions: &mut ComponentStore<TransitionTo>,
) {
    assert!(child_of.insert(edge, ChildOf(source)));
    assert!(actions.insert(edge, ActionEdge::new(label, target).unwrap()));
    assert!(transitions.insert(edge, TransitionTo(target)));
}

#[test]
fn information_set_fixtures_group_nodes_with_matching_action_labels() {
    let first = entity(1);
    let second = entity(2);
    let call_target = entity(3);
    let fold_target = entity(4);
    let mut nodes = ComponentStore::new();
    let mut information_sets = ComponentStore::new();
    let mut child_of = ComponentStore::new();
    let mut actions = ComponentStore::new();
    let mut transitions = ComponentStore::new();

    assert!(nodes.insert(first, ExtensiveNode::decision(0).unwrap()));
    assert!(nodes.insert(second, ExtensiveNode::decision(0).unwrap()));
    assert!(nodes.insert(call_target, ExtensiveNode::terminal()));
    assert!(nodes.insert(fold_target, ExtensiveNode::terminal()));
    assert!(information_sets.insert(first, InformationSet::new(7, 0).unwrap()));
    assert!(information_sets.insert(second, InformationSet::new(7, 0).unwrap()));
    add_action(
        first,
        entity(10),
        "call",
        call_target,
        &mut child_of,
        &mut actions,
        &mut transitions,
    );
    add_action(
        first,
        entity(11),
        "fold",
        fold_target,
        &mut child_of,
        &mut actions,
        &mut transitions,
    );
    add_action(
        second,
        entity(12),
        "call",
        call_target,
        &mut child_of,
        &mut actions,
        &mut transitions,
    );
    add_action(
        second,
        entity(13),
        "fold",
        fold_target,
        &mut child_of,
        &mut actions,
        &mut transitions,
    );

    let stores = ExtensiveTraversalStores {
        child_of: &child_of,
        actions: &actions,
        transitions: &transitions,
    };

    let fixtures = information_set_fixtures(&nodes, &information_sets, stores).unwrap();

    assert_eq!(fixtures.len(), 1);
    assert_eq!(fixtures[0].id, 7);
    assert_eq!(fixtures[0].player, 0);
    assert_eq!(fixtures[0].nodes, vec![first, second]);
    assert_eq!(
        fixtures[0].actions,
        vec!["call".to_string(), "fold".to_string()]
    );
}

#[test]
fn information_set_fixtures_reject_player_mismatch() {
    let node = entity(20);
    let mut nodes = ComponentStore::new();
    let mut information_sets = ComponentStore::new();
    let child_of = ComponentStore::new();
    let actions = ComponentStore::new();
    let transitions = ComponentStore::new();

    assert!(nodes.insert(node, ExtensiveNode::decision(0).unwrap()));
    assert!(information_sets.insert(node, InformationSet::new(8, 1).unwrap()));

    let stores = ExtensiveTraversalStores {
        child_of: &child_of,
        actions: &actions,
        transitions: &transitions,
    };

    assert_eq!(
        information_set_fixtures(&nodes, &information_sets, stores),
        Err(ExtensiveFormError::InformationSetPlayerMismatch {
            information_set: 8,
            entity: node,
            expected_player: 1,
            actual_player: 0
        })
    );
}

#[test]
fn information_set_fixtures_reject_inconsistent_action_labels() {
    let first = entity(30);
    let second = entity(31);
    let target = entity(32);
    let mut nodes = ComponentStore::new();
    let mut information_sets = ComponentStore::new();
    let mut child_of = ComponentStore::new();
    let mut actions = ComponentStore::new();
    let mut transitions = ComponentStore::new();

    assert!(nodes.insert(first, ExtensiveNode::decision(0).unwrap()));
    assert!(nodes.insert(second, ExtensiveNode::decision(0).unwrap()));
    assert!(nodes.insert(target, ExtensiveNode::terminal()));
    assert!(information_sets.insert(first, InformationSet::new(9, 0).unwrap()));
    assert!(information_sets.insert(second, InformationSet::new(9, 0).unwrap()));
    add_action(
        first,
        entity(33),
        "call",
        target,
        &mut child_of,
        &mut actions,
        &mut transitions,
    );
    add_action(
        second,
        entity(34),
        "raise",
        target,
        &mut child_of,
        &mut actions,
        &mut transitions,
    );

    let stores = ExtensiveTraversalStores {
        child_of: &child_of,
        actions: &actions,
        transitions: &transitions,
    };

    assert_eq!(
        information_set_fixtures(&nodes, &information_sets, stores),
        Err(ExtensiveFormError::InconsistentInformationSetActions { information_set: 9 })
    );
}
