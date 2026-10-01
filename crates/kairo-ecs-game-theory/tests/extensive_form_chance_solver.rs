#![cfg(feature = "graph-relations")]
#![allow(unused_mut)]

use kairo_ecs_game_theory::extensive_form::{
    ActionEdge, ChanceOutcome, ChanceTraversalStores, ChanceWeightedBackwardInductionSolver,
    ExtensiveNode, TerminalUtility,
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

#[allow(clippy::too_many_arguments)]
fn add_chance(
    source: EntityId,
    edge: EntityId,
    label: &str,
    probability: f64,
    target: EntityId,
    child_of: &mut ComponentStore<ChildOf>,
    outcomes: &mut ComponentStore<ChanceOutcome>,
    transitions: &mut ComponentStore<TransitionTo>,
) {
    assert!(child_of.insert(edge, ChildOf(source)));
    assert!(outcomes.insert(
        edge,
        ChanceOutcome::new(label, probability, target).unwrap()
    ));
    assert!(transitions.insert(edge, TransitionTo(target)));
}

#[test]
fn chance_weighted_solver_aggregates_expected_payoffs_for_chance_root() {
    let root = entity(1);
    let heads = entity(2);
    let tails = entity(3);
    let heads_edge = entity(4);
    let tails_edge = entity(5);
    let mut nodes = ComponentStore::new();
    let mut terminals = ComponentStore::new();
    let mut child_of = ComponentStore::new();
    let mut actions = ComponentStore::new();
    let mut transitions = ComponentStore::new();
    let mut outcomes = ComponentStore::new();

    assert!(nodes.insert(root, ExtensiveNode::chance()));
    assert!(nodes.insert(heads, ExtensiveNode::terminal()));
    assert!(nodes.insert(tails, ExtensiveNode::terminal()));
    assert!(terminals.insert(
        heads,
        TerminalUtility::new(vec![utility(4.0), utility(1.0)]).unwrap()
    ));
    assert!(terminals.insert(
        tails,
        TerminalUtility::new(vec![utility(0.0), utility(3.0)]).unwrap()
    ));
    add_chance(
        root,
        heads_edge,
        "heads",
        0.25,
        heads,
        &mut child_of,
        &mut outcomes,
        &mut transitions,
    );
    add_chance(
        root,
        tails_edge,
        "tails",
        0.75,
        tails,
        &mut child_of,
        &mut outcomes,
        &mut transitions,
    );

    let stores = ChanceTraversalStores {
        child_of: &mut child_of,
        actions: &mut actions,
        chance_outcomes: &mut outcomes,
        transitions: &mut transitions,
    };

    let outcome =
        ChanceWeightedBackwardInductionSolver::solve(root, &nodes, &terminals, stores).unwrap();

    assert_eq!(outcome.actions, Vec::<String>::new());
    assert_eq!(outcome.terminal, None);
    assert_eq!(outcome.expected_payoffs, vec![utility(1.0), utility(2.5)]);
}

#[test]
fn chance_weighted_solver_uses_chance_expectation_inside_decision_nodes() {
    let root = entity(10);
    let risky = entity(11);
    let safe = entity(12);
    let risky_heads = entity(13);
    let risky_tails = entity(14);
    let safe_terminal = entity(15);
    let enter_edge = entity(16);
    let safe_edge = entity(17);
    let heads_edge = entity(18);
    let tails_edge = entity(19);
    let mut nodes = ComponentStore::new();
    let mut terminals = ComponentStore::new();
    let mut child_of = ComponentStore::new();
    let mut actions = ComponentStore::new();
    let mut transitions = ComponentStore::new();
    let mut outcomes = ComponentStore::new();

    assert!(nodes.insert(root, ExtensiveNode::decision(0).unwrap()));
    assert!(nodes.insert(risky, ExtensiveNode::chance()));
    assert!(nodes.insert(safe, ExtensiveNode::terminal()));
    assert!(nodes.insert(risky_heads, ExtensiveNode::terminal()));
    assert!(nodes.insert(risky_tails, ExtensiveNode::terminal()));
    assert!(nodes.insert(safe_terminal, ExtensiveNode::terminal()));
    assert!(terminals.insert(
        risky_heads,
        TerminalUtility::new(vec![utility(10.0)]).unwrap()
    ));
    assert!(terminals.insert(
        risky_tails,
        TerminalUtility::new(vec![utility(0.0)]).unwrap()
    ));
    assert!(terminals.insert(
        safe_terminal,
        TerminalUtility::new(vec![utility(3.0)]).unwrap()
    ));
    add_action(
        root,
        enter_edge,
        "enter",
        risky,
        &mut child_of,
        &mut actions,
        &mut transitions,
    );
    add_action(
        root,
        safe_edge,
        "safe",
        safe_terminal,
        &mut child_of,
        &mut actions,
        &mut transitions,
    );
    add_chance(
        risky,
        heads_edge,
        "heads",
        0.4,
        risky_heads,
        &mut child_of,
        &mut outcomes,
        &mut transitions,
    );
    add_chance(
        risky,
        tails_edge,
        "tails",
        0.6,
        risky_tails,
        &mut child_of,
        &mut outcomes,
        &mut transitions,
    );

    let stores = ChanceTraversalStores {
        child_of: &mut child_of,
        actions: &mut actions,
        chance_outcomes: &mut outcomes,
        transitions: &mut transitions,
    };

    let outcome =
        ChanceWeightedBackwardInductionSolver::solve(root, &nodes, &terminals, stores).unwrap();

    assert_eq!(outcome.actions, vec!["enter".to_string()]);
    assert_eq!(outcome.terminal, None);
    assert_eq!(outcome.expected_payoffs, vec![utility(4.0)]);
}
