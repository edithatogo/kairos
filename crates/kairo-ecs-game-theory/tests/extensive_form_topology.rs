#![cfg(feature = "graph-relations")]

use kairo_ecs_game_theory::extensive_form::{
    validate_extensive_form_topology, ActionEdge, ExtensiveFormError, ExtensiveFormTopology,
    ExtensiveNode, TerminalUtility,
};
use kairo_ecs_game_theory::graph_relations::ChildOf;
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
fn topology_validation_accepts_reachable_root_to_terminal_tree() {
    let root = entity(1);
    let terminal = entity(2);
    let mut nodes = ComponentStore::new();
    let mut child_of = ComponentStore::new();
    let mut actions = ComponentStore::new();
    let chance_outcomes = ComponentStore::new();
    let mut terminals = ComponentStore::new();

    assert!(nodes.insert(root, ExtensiveNode::decision(0).unwrap()));
    assert!(nodes.insert(terminal, ExtensiveNode::terminal()));
    assert!(child_of.insert(terminal, ChildOf(root)));
    assert!(actions.insert(root, ActionEdge::new("stop", terminal).unwrap()));
    assert!(terminals.insert(terminal, TerminalUtility::new(vec![utility(1.0)]).unwrap()));

    let topology = ExtensiveFormTopology {
        root,
        nodes: &nodes,
        child_of: &child_of,
        actions: &actions,
        chance_outcomes: &chance_outcomes,
        terminals: &terminals,
    };

    assert_eq!(validate_extensive_form_topology(topology), Ok(()));
}

#[test]
fn topology_validation_rejects_missing_action_targets() {
    let root = entity(10);
    let missing = entity(99);
    let mut nodes = ComponentStore::new();
    let child_of = ComponentStore::new();
    let mut actions = ComponentStore::new();
    let chance_outcomes = ComponentStore::new();
    let terminals = ComponentStore::new();

    assert!(nodes.insert(root, ExtensiveNode::decision(0).unwrap()));
    assert!(actions.insert(root, ActionEdge::new("missing", missing).unwrap()));

    let topology = ExtensiveFormTopology {
        root,
        nodes: &nodes,
        child_of: &child_of,
        actions: &actions,
        chance_outcomes: &chance_outcomes,
        terminals: &terminals,
    };

    assert_eq!(
        validate_extensive_form_topology(topology),
        Err(ExtensiveFormError::MissingNode { entity: missing })
    );
}

#[test]
fn topology_validation_rejects_child_cycles() {
    let root = entity(20);
    let child = entity(21);
    let mut nodes = ComponentStore::new();
    let mut child_of = ComponentStore::new();
    let actions = ComponentStore::new();
    let chance_outcomes = ComponentStore::new();
    let terminals = ComponentStore::new();

    assert!(nodes.insert(root, ExtensiveNode::decision(0).unwrap()));
    assert!(nodes.insert(child, ExtensiveNode::decision(1).unwrap()));
    assert!(child_of.insert(child, ChildOf(root)));
    assert!(child_of.insert(root, ChildOf(child)));

    let topology = ExtensiveFormTopology {
        root,
        nodes: &nodes,
        child_of: &child_of,
        actions: &actions,
        chance_outcomes: &chance_outcomes,
        terminals: &terminals,
    };

    assert_eq!(
        validate_extensive_form_topology(topology),
        Err(ExtensiveFormError::CycleDetected { entity: root })
    );
}

#[test]
fn topology_validation_rejects_terminal_nodes_without_terminal_utility() {
    let root = entity(30);
    let mut nodes = ComponentStore::new();
    let child_of = ComponentStore::new();
    let actions = ComponentStore::new();
    let chance_outcomes = ComponentStore::new();
    let terminals = ComponentStore::new();

    assert!(nodes.insert(root, ExtensiveNode::terminal()));

    let topology = ExtensiveFormTopology {
        root,
        nodes: &nodes,
        child_of: &child_of,
        actions: &actions,
        chance_outcomes: &chance_outcomes,
        terminals: &terminals,
    };

    assert_eq!(
        validate_extensive_form_topology(topology),
        Err(ExtensiveFormError::MissingTerminalUtility { entity: root })
    );
}
