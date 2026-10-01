#![cfg(feature = "graph-relations")]

use kairo_ecs_game_theory::extensive_form::{
    ActionEdge, BackwardInductionSolver, ExtensiveNode, ExtensiveTraversalStores, TerminalUtility,
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
fn backward_induction_selects_best_root_action_for_active_player() {
    let root = entity(1);
    let left_edge = entity(2);
    let right_edge = entity(3);
    let left_terminal = entity(4);
    let right_terminal = entity(5);
    let mut nodes = ComponentStore::new();
    let mut terminals = ComponentStore::new();
    let mut child_of = ComponentStore::new();
    let mut actions = ComponentStore::new();
    let mut transitions = ComponentStore::new();

    assert!(nodes.insert(root, ExtensiveNode::decision(0).unwrap()));
    assert!(nodes.insert(left_terminal, ExtensiveNode::terminal()));
    assert!(nodes.insert(right_terminal, ExtensiveNode::terminal()));
    assert!(terminals.insert(
        left_terminal,
        TerminalUtility::new(vec![utility(1.0)]).unwrap()
    ));
    assert!(terminals.insert(
        right_terminal,
        TerminalUtility::new(vec![utility(3.0)]).unwrap()
    ));
    add_action(
        root,
        left_edge,
        "left",
        left_terminal,
        &mut child_of,
        &mut actions,
        &mut transitions,
    );
    add_action(
        root,
        right_edge,
        "right",
        right_terminal,
        &mut child_of,
        &mut actions,
        &mut transitions,
    );

    let stores = ExtensiveTraversalStores {
        child_of: &child_of,
        actions: &actions,
        transitions: &transitions,
    };

    let outcome = BackwardInductionSolver::solve(root, &nodes, &terminals, stores).unwrap();

    assert_eq!(outcome.actions, vec!["right".to_string()]);
    assert_eq!(outcome.terminal, right_terminal);
    assert_eq!(outcome.payoffs, vec![utility(3.0)]);
}

#[test]
fn backward_induction_solves_nested_subgames_by_current_player() {
    let root = entity(10);
    let player_one = entity(11);
    let outside = entity(12);
    let accept = entity(13);
    let reject = entity(14);
    let enter_edge = entity(15);
    let outside_edge = entity(16);
    let accept_edge = entity(17);
    let reject_edge = entity(18);
    let mut nodes = ComponentStore::new();
    let mut terminals = ComponentStore::new();
    let mut child_of = ComponentStore::new();
    let mut actions = ComponentStore::new();
    let mut transitions = ComponentStore::new();

    assert!(nodes.insert(root, ExtensiveNode::decision(0).unwrap()));
    assert!(nodes.insert(player_one, ExtensiveNode::decision(1).unwrap()));
    for terminal in [outside, accept, reject] {
        assert!(nodes.insert(terminal, ExtensiveNode::terminal()));
    }
    assert!(terminals.insert(
        outside,
        TerminalUtility::new(vec![utility(1.0), utility(1.0)]).unwrap()
    ));
    assert!(terminals.insert(
        accept,
        TerminalUtility::new(vec![utility(3.0), utility(2.0)]).unwrap()
    ));
    assert!(terminals.insert(
        reject,
        TerminalUtility::new(vec![utility(0.0), utility(0.0)]).unwrap()
    ));
    add_action(
        root,
        enter_edge,
        "enter",
        player_one,
        &mut child_of,
        &mut actions,
        &mut transitions,
    );
    add_action(
        root,
        outside_edge,
        "outside",
        outside,
        &mut child_of,
        &mut actions,
        &mut transitions,
    );
    add_action(
        player_one,
        accept_edge,
        "accept",
        accept,
        &mut child_of,
        &mut actions,
        &mut transitions,
    );
    add_action(
        player_one,
        reject_edge,
        "reject",
        reject,
        &mut child_of,
        &mut actions,
        &mut transitions,
    );

    let stores = ExtensiveTraversalStores {
        child_of: &child_of,
        actions: &actions,
        transitions: &transitions,
    };

    let outcome = BackwardInductionSolver::solve(root, &nodes, &terminals, stores).unwrap();

    assert_eq!(
        outcome.actions,
        vec!["enter".to_string(), "accept".to_string()]
    );
    assert_eq!(outcome.terminal, accept);
    assert_eq!(outcome.payoffs, vec![utility(3.0), utility(2.0)]);
}

#[test]
fn backward_induction_preserves_first_best_action_on_ties() {
    let root = entity(30);
    let first_edge = entity(31);
    let second_edge = entity(32);
    let first_terminal = entity(33);
    let second_terminal = entity(34);
    let mut nodes = ComponentStore::new();
    let mut terminals = ComponentStore::new();
    let mut child_of = ComponentStore::new();
    let mut actions = ComponentStore::new();
    let mut transitions = ComponentStore::new();

    assert!(nodes.insert(root, ExtensiveNode::decision(0).unwrap()));
    assert!(nodes.insert(first_terminal, ExtensiveNode::terminal()));
    assert!(nodes.insert(second_terminal, ExtensiveNode::terminal()));
    assert!(terminals.insert(
        first_terminal,
        TerminalUtility::new(vec![utility(2.0)]).unwrap()
    ));
    assert!(terminals.insert(
        second_terminal,
        TerminalUtility::new(vec![utility(2.0)]).unwrap()
    ));
    add_action(
        root,
        first_edge,
        "first",
        first_terminal,
        &mut child_of,
        &mut actions,
        &mut transitions,
    );
    add_action(
        root,
        second_edge,
        "second",
        second_terminal,
        &mut child_of,
        &mut actions,
        &mut transitions,
    );

    let stores = ExtensiveTraversalStores {
        child_of: &child_of,
        actions: &actions,
        transitions: &transitions,
    };

    let outcome = BackwardInductionSolver::solve(root, &nodes, &terminals, stores).unwrap();

    assert_eq!(outcome.actions, vec!["first".to_string()]);
    assert_eq!(outcome.terminal, first_terminal);
}
