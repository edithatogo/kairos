#![cfg(feature = "graph-relations")]

use kairo_ecs_game_theory::extensive_form::{
    extensive_form_paths, outgoing_action_edges, ActionEdge, ExtensiveFormError, ExtensiveNode,
    ExtensiveTraversalStores, TerminalUtility,
};
use kairo_ecs_game_theory::graph_relations::{ChildOf, TransitionTo};
use kairo_ecs_game_theory::normal_form::Utility;
use kairo_ecs_state::ComponentStore;
use kairo_ecs_types::EntityId;

fn entity(index: u64) -> EntityId {
    EntityId::new(index, 0)
}

fn utility(value: f64) -> Utility {
    Utility::new(value).unwrap()
}

#[test]
fn traversal_reads_action_edges_from_child_entities() {
    let root = entity(1);
    let left_edge = entity(2);
    let right_edge = entity(3);
    let left_terminal = entity(4);
    let right_terminal = entity(5);
    let mut child_of = ComponentStore::new();
    let mut actions = ComponentStore::new();
    let mut transitions = ComponentStore::new();

    assert!(child_of.insert(left_edge, ChildOf(root)));
    assert!(child_of.insert(right_edge, ChildOf(root)));
    assert!(actions.insert(left_edge, ActionEdge::new("left", left_terminal).unwrap()));
    assert!(actions.insert(
        right_edge,
        ActionEdge::new("right", right_terminal).unwrap()
    ));
    assert!(transitions.insert(left_edge, TransitionTo(left_terminal)));
    assert!(transitions.insert(right_edge, TransitionTo(right_terminal)));

    let stores = ExtensiveTraversalStores {
        child_of: &child_of,
        actions: &actions,
        transitions: &transitions,
    };

    let edges = outgoing_action_edges(root, stores).unwrap();

    assert_eq!(edges.len(), 2);
    assert_eq!(edges[0].source, root);
    assert_eq!(edges[0].edge, left_edge);
    assert_eq!(edges[0].target, left_terminal);
    assert_eq!(edges[0].label, "left");
    assert_eq!(edges[1].edge, right_edge);
    assert_eq!(edges[1].target, right_terminal);
    assert_eq!(edges[1].label, "right");
}

#[test]
fn traversal_rejects_action_transition_target_mismatch() {
    let root = entity(10);
    let edge = entity(11);
    let action_target = entity(12);
    let transition_target = entity(13);
    let mut child_of = ComponentStore::new();
    let mut actions = ComponentStore::new();
    let mut transitions = ComponentStore::new();

    assert!(child_of.insert(edge, ChildOf(root)));
    assert!(actions.insert(edge, ActionEdge::new("bad", action_target).unwrap()));
    assert!(transitions.insert(edge, TransitionTo(transition_target)));

    let stores = ExtensiveTraversalStores {
        child_of: &child_of,
        actions: &actions,
        transitions: &transitions,
    };

    assert_eq!(
        outgoing_action_edges(root, stores),
        Err(ExtensiveFormError::TransitionTargetMismatch {
            edge,
            action_target,
            transition_target
        })
    );
}

#[test]
fn traversal_paths_walk_to_terminals_in_edge_order() {
    let root = entity(20);
    let left_edge = entity(21);
    let right_edge = entity(22);
    let left_terminal = entity(23);
    let right_terminal = entity(24);
    let mut nodes = ComponentStore::new();
    let mut child_of = ComponentStore::new();
    let mut actions = ComponentStore::new();
    let mut transitions = ComponentStore::new();
    let mut terminals = ComponentStore::new();

    assert!(nodes.insert(root, ExtensiveNode::decision(0).unwrap()));
    assert!(nodes.insert(left_terminal, ExtensiveNode::terminal()));
    assert!(nodes.insert(right_terminal, ExtensiveNode::terminal()));
    assert!(child_of.insert(left_edge, ChildOf(root)));
    assert!(child_of.insert(right_edge, ChildOf(root)));
    assert!(actions.insert(left_edge, ActionEdge::new("left", left_terminal).unwrap()));
    assert!(actions.insert(
        right_edge,
        ActionEdge::new("right", right_terminal).unwrap()
    ));
    assert!(transitions.insert(left_edge, TransitionTo(left_terminal)));
    assert!(transitions.insert(right_edge, TransitionTo(right_terminal)));
    assert!(terminals.insert(
        left_terminal,
        TerminalUtility::new(vec![utility(2.0)]).unwrap()
    ));
    assert!(terminals.insert(
        right_terminal,
        TerminalUtility::new(vec![utility(3.0)]).unwrap()
    ));

    let stores = ExtensiveTraversalStores {
        child_of: &child_of,
        actions: &actions,
        transitions: &transitions,
    };

    let paths = extensive_form_paths(root, &nodes, &terminals, stores).unwrap();

    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0].terminal, left_terminal);
    assert_eq!(paths[0].actions, vec!["left".to_string()]);
    assert_eq!(paths[0].payoffs, vec![utility(2.0)]);
    assert_eq!(paths[1].terminal, right_terminal);
    assert_eq!(paths[1].actions, vec!["right".to_string()]);
    assert_eq!(paths[1].payoffs, vec![utility(3.0)]);
}
